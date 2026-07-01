#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const upstreamDir = process.argv[2] || "/tmp/disk-nix-disko-upstream/example";
const outDir = process.argv[3] || "examples/disko";
const testDisks = ["/dev/sdb", "/dev/sdc", "/dev/sdd", "/dev/sde", "/dev/sdf"];
const coalescedDiskSizeMiB = 16 * 1024;

function stableName(value) {
  return value
    .replace(/\.nix$/, "")
    .replace(/[/-]+/g, "-")
    .replace(/[^A-Za-z0-9._-]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

async function findNixFiles(dir, prefix = "") {
  const entries = await readdir(path.join(dir, prefix), { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const relative = path.join(prefix, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await findNixFiles(dir, relative)));
    } else if (entry.isFile() && entry.name.endsWith(".nix")) {
      files.push(relative);
    }
  }
  return files.sort();
}

function extractBalancedValue(source, marker) {
  const markerIndex = source.indexOf(marker);
  if (markerIndex < 0) {
    return null;
  }
  const equalsIndex = source.indexOf("=", markerIndex + marker.length);
  if (equalsIndex < 0) {
    return null;
  }
  const start = source.indexOf("{", equalsIndex);
  if (start < 0) {
    return null;
  }
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = start; index < source.length; index += 1) {
    const char = source[index];
    if (inString) {
      escaped = char === "\\" && !escaped;
      if (char === '"' && !escaped) {
        inString = false;
      } else if (char !== "\\") {
        escaped = false;
      }
      continue;
    }
    if (char === '"') {
      inString = true;
      continue;
    }
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(start, index + 1);
      }
    }
  }
  return null;
}

function partitionPath(device, number) {
  return /[0-9]$/.test(device) ? `${device}p${number}` : `${device}${number}`;
}

function sizeToMiB(size) {
  if (typeof size !== "string") {
    return null;
  }
  const match = /^(-?)([0-9]+)([KMGTP]i?B?|%)?$/i.exec(size.trim());
  if (!match || match[3] === "%") {
    return null;
  }
  const sign = match[1] === "-" ? -1 : 1;
  const value = Number(match[2]);
  const unit = (match[3] || "MiB").toLowerCase();
  const factor =
    unit.startsWith("k") ? 1 / 1024
    : unit.startsWith("g") ? 1024
    : unit.startsWith("t") ? 1024 * 1024
    : unit.startsWith("p") ? 1024 * 1024 * 1024
    : 1;
  return sign * value * factor;
}

function formatMiB(value) {
  if (!Number.isFinite(value)) {
    return undefined;
  }
  return `${Math.max(1, Math.round(value))}MiB`;
}

function objectEntries(value) {
  return value && typeof value === "object" && !Array.isArray(value)
    ? Object.entries(value)
    : [];
}

function asArray(value) {
  return Array.isArray(value) ? value : [];
}

function sorted(value) {
  if (Array.isArray(value)) {
    return value.map(sorted);
  }
  if (!value || typeof value !== "object") {
    return value;
  }
  return Object.fromEntries(
    Object.entries(value)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, item]) => [key, sorted(item)]),
  );
}

function evalNixJson(expr) {
  return JSON.parse(
    execFileSync("nix", ["eval", "--impure", "--json", "--expr", expr], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
}

function evalDiskoDevices(file, sourceText) {
  const embeddedDevices = extractBalancedValue(sourceText, "cfg.disko.devices");
  if (embeddedDevices) {
    return {
      devices: evalNixJson(`let cfg.disko.devices = ${embeddedDevices}; in cfg.disko.devices`),
      notes: [
        "embedded cfg.disko.devices was extracted from a NixOS configuration example",
      ],
    };
  }
  const expr = `
let
  pkgs = import <nixpkgs> {};
  lib = pkgs.lib;
  imported = import ${JSON.stringify(file)};
  cfg =
    if builtins.isFunction imported then
      imported { inherit lib; disks = [ ${testDisks.map((disk) => JSON.stringify(disk)).join(" ")} ]; }
    else
      imported;
in cfg.disko.devices
`;
  try {
    return {
      devices: evalNixJson(expr),
      notes: [],
    };
  } catch (error) {
    throw error;
  }
}

function ensureCollection(spec, name) {
  if (name === "luks.devices") {
    spec.luks ??= {};
    spec.luks.devices ??= {};
    return spec.luks.devices;
  }
  spec[name] ??= {};
  return spec[name];
}

function addObject(spec, collection, name, value) {
  ensureCollection(spec, collection)[name] = value;
}

function metadata(source, extra = {}) {
  return {
    diskoSource: source,
    ...extra,
  };
}

function mapperPath(name) {
  return `/dev/mapper/${name}`;
}

function translateFilesystem(spec, manifest, source, name, device, content) {
  const fsType = content.format || content.type || "unknown";
  const entry = {
    device,
    fsType,
    mountpoint: content.mountpoint ?? name,
    operation: "format",
    preserveData: false,
    metadata: metadata(source, {
      diskoType: content.type,
      extraArgs: content.extraArgs,
      extraFormatArgs: content.extraFormatArgs,
    }),
  };
  if (content.mountOptions) {
    entry.options = content.mountOptions;
  }
  if (content.mountpoint === null) {
    entry.mountpoint = name;
    entry.metadata.unmounted = true;
    manifest.notes.push(`${source}: filesystem ${name} has mountpoint = null; translated as format-only`);
  }
  addObject(spec, "filesystems", name, entry);
}

function translateSwap(spec, source, name, device, content) {
  const entry = {
    device,
    operation: "format",
    preserveData: false,
    metadata: metadata(source, {
      randomEncryption: content.randomEncryption,
      resumeDevice: content.resumeDevice,
      discardPolicy: content.discardPolicy,
    }),
  };
  const properties = {};
  if (content.priority !== undefined) {
    properties.priority = String(content.priority);
  }
  if (Object.keys(properties).length > 0) {
    entry.properties = properties;
  }
  addObject(spec, "swaps", name, entry);
}

function translateBtrfs(spec, source, name, device, content) {
  translateFilesystem(spec, { notes: [] }, source, name, device, {
    ...content,
    type: "filesystem",
    format: "btrfs",
  });
  for (const [subvol, subvolSpec] of objectEntries(content.subvolumes)) {
    addObject(spec, "btrfsSubvolumes", `${name}:${subvol}`, {
      operation: "create",
      target: subvol,
      mountpoint: subvolSpec.mountpoint,
      options: subvolSpec.mountOptions,
      metadata: metadata(source, {
        parentFilesystem: name,
        swap: subvolSpec.swap,
      }),
    });
  }
}

function translateBcachefs(spec, manifest, source, name, device, content) {
  const filesystem = content.filesystem || name;
  const entryName = `bcachefs:${filesystem}`;
  const existing = ensureCollection(spec, "filesystems")[entryName];
  if (existing) {
    existing.addDevices ??= [];
    existing.addDevices.push(device);
    return;
  }
  addObject(spec, "filesystems", entryName, {
    device,
    fsType: "bcachefs",
    mountpoint: content.mountpoint || `/${filesystem}`,
    operation: "format",
    preserveData: false,
    metadata: metadata(source, {
      label: content.label,
      extraFormatArgs: content.extraFormatArgs,
      diskoFilesystem: filesystem,
    }),
  });
  manifest.notes.push(`${source}: bcachefs multi-device membership is recorded as filesystem addDevices metadata`);
}

function translateLuks(spec, manifest, source, name, device, content, recurse) {
  addObject(spec, "luks.devices", name, {
    device,
    operation: "format",
    preserveData: false,
    metadata: metadata(source, {
      settings: content.settings,
      extraFormatArgs: content.extraFormatArgs,
      additionalKeyFiles: content.additionalKeyFiles,
    }),
  });
  recurse(mapperPath(name), `${source}:${name}`, content.content);
  manifest.notes.push(`${source}: LUKS settings are preserved in metadata for operator review`);
}

function translateLvmPv(spec, source, name, device, content) {
  addObject(spec, "physicalVolumes", device, {
    operation: "create",
    device,
    metadata: metadata(source),
  });
  if (content.vg) {
    const groups = ensureCollection(spec, "volumeGroups");
    groups[content.vg] ??= {
      operation: "create",
      device,
      devices: [],
      metadata: metadata(source),
    };
    groups[content.vg].devices.push(device);
    groups[content.vg].device ??= device;
  }
}

function translateMdMember(manifest, source, name, device, content) {
  const mdName = content.name || name;
  manifest.mdMembers[mdName] ??= [];
  manifest.mdMembers[mdName].push(device);
  manifest.notes.push(`${source}: partition ${device} contributes to MD RAID ${mdName}`);
}

function translateZfsMember(manifest, source, name, device, content) {
  const pool = content.pool || name;
  manifest.zfsMembers[pool] ??= [];
  manifest.zfsMembers[pool].push(device);
  manifest.notes.push(`${source}: device ${device} contributes to ZFS pool ${pool}`);
}

function translateContent(spec, manifest, source, name, device, content) {
  if (!content || typeof content !== "object") {
    return;
  }
  const recurse = (nextDevice, nextName, nextContent) =>
    translateContent(spec, manifest, nextName, nextName, nextDevice, nextContent);

  switch (content.type) {
    case "filesystem":
      translateFilesystem(spec, manifest, source, name, device, content);
      break;
    case "btrfs":
      translateBtrfs(spec, source, name, device, content);
      break;
    case "bcachefs":
      translateBcachefs(spec, manifest, source, name, device, content);
      break;
    case "swap":
      translateSwap(spec, source, name, device, content);
      break;
    case "luks":
      translateLuks(spec, manifest, source, content.name || name, device, content, recurse);
      break;
    case "lvm_pv":
      translateLvmPv(spec, source, name, device, content);
      break;
    case "mdraid":
      translateMdMember(manifest, source, name, device, content);
      break;
    case "zfs":
      translateZfsMember(manifest, source, name, device, content);
      break;
    case "gpt":
    case "mbr":
    case "table":
      translatePartitionTable(spec, manifest, source, device, content);
      break;
    default:
      manifest.unsupported.push(`${source}: unsupported nested content type ${content.type}`);
  }
}

function translatePartitionTable(spec, manifest, source, device, content) {
  manifest.partitionState ??= {};
  manifest.partitionState[device] ??= { number: 0, cursorMiB: 1 };
  const state = manifest.partitionState[device];
  const partitions = Array.isArray(content.partitions)
    ? content.partitions.map((partition, index) => [
        partition.name || `partition-${index + 1}`,
        partition,
      ])
    : objectEntries(content.partitions);
  for (const [index, [name, partition]] of partitions.entries()) {
    const number = state.number + 1;
    const partDevice = partitionPath(device, number);
    const start = partition.start || formatMiB(state.cursorMiB);
    let end = partition.end;
    if (!end && partition.size === "100%") {
      const nextPartitionWithStart = partitions
        .slice(index + 1)
        .map(([, nextPartition]) => nextPartition.start)
        .find((nextStart) => typeof nextStart === "string");
      end = nextPartitionWithStart
        || (index + 1 < partitions.length || manifest.coalescedPhysicalDisks?.includes(device)
        ? formatMiB(state.cursorMiB + coalescedDiskSizeMiB)
        : "100%");
    } else if (!end && partition.size) {
      const sizeMiB = sizeToMiB(partition.size);
      if (sizeMiB !== null) {
        end = formatMiB(state.cursorMiB + sizeMiB);
      }
    }
    const endMiB = sizeToMiB(end);
    if (end === "100%") {
      state.cursorMiB += coalescedDiskSizeMiB;
    } else if (endMiB !== null && endMiB > state.cursorMiB) {
      state.cursorMiB = endMiB;
    }
    state.number = number;
    addObject(spec, "partitions", `${source}:${name}`, {
      operation: "create",
      device,
      target: partDevice,
      partitionNumber: String(number),
      start,
      end,
      size: partition.size,
      partitionType: partition.partType || partition["part-type"] || "primary",
      name: partition.name || name,
      metadata: metadata(source, {
        priority: partition.priority,
        attributes: partition.attributes,
        hybrid: partition.hybrid,
        uuid: partition.uuid,
        diskoPartitionType: partition.type,
      }),
    });
    translateContent(spec, manifest, source, `${source}:${name}`, partDevice, partition.content);
  }
}

function translateTopLevel(spec, manifest, devices) {
  const originalDisks = Object.values(devices.disk || {}).map((disk) => disk.device);
  const mapping = new Map();
  originalDisks.forEach((device, index) => {
    mapping.set(device, testDisks[index % testDisks.length]);
  });
  manifest.originalDisks = originalDisks;
  manifest.testDisks = [...new Set([...mapping.values()])];
  const physicalCounts = {};
  for (const physical of mapping.values()) {
    physicalCounts[physical] = (physicalCounts[physical] || 0) + 1;
  }
  manifest.coalescedPhysicalDisks = Object.entries(physicalCounts)
    .filter(([, count]) => count > 1)
    .map(([device]) => device);
  if (originalDisks.length > testDisks.length) {
    manifest.notes.push(
      `source uses ${originalDisks.length} disks; mapped onto ${testDisks.length} test disks with ${coalescedDiskSizeMiB}MiB logical slices`,
    );
  }

  for (const [name, disk] of objectEntries(devices.disk)) {
    const testDevice = mapping.get(disk.device) || testDisks[0];
    const disks = ensureCollection(spec, "disks");
    disks[testDevice] ??= {
      operation: "create",
      partitionType: disk.content?.format || disk.content?.type || "gpt",
      metadata: metadata(name, {
        originalDevices: [],
        diskoDiskNames: [],
      }),
    };
    disks[testDevice].metadata.originalDevices.push(disk.device);
    disks[testDevice].metadata.diskoDiskNames.push(name);
    translateContent(spec, manifest, name, name, testDevice, disk.content);
  }

  for (const [name, vg] of objectEntries(devices.lvm_vg)) {
    const group = ensureCollection(spec, "volumeGroups")[name] ?? {
      operation: "create",
      devices: [],
      metadata: metadata(`lvm_vg:${name}`),
    };
    if (!group.device && Array.isArray(group.devices) && group.devices.length > 0) {
      group.device = group.devices[0];
    }
    ensureCollection(spec, "volumeGroups")[name] = group;
    for (const [lvName, lv] of objectEntries(vg.lvs)) {
      const target = `${name}/${lvName}`;
      addObject(spec, "volumes", target, {
        operation: "create",
        target,
        desiredSize: lv.size,
        metadata: metadata(`lvm_vg:${name}`, {
          lvmType: lv.lvm_type,
        }),
      });
      translateContent(spec, manifest, `lvm:${target}`, target, `/dev/${name}/${lvName}`, lv.content);
    }
  }

  for (const [name, md] of objectEntries(devices.mdadm)) {
    const devicesForArray = manifest.mdMembers[name] || [];
    addObject(spec, "mdRaids", name, {
      operation: "create",
      target: `/dev/md/${name}`,
      devices: devicesForArray,
      level: String(md.level ?? "1"),
      metadata: metadata(name, {
        extraArgs: md.extraArgs,
        metadata: md.metadata,
      }),
    });
    translateContent(spec, manifest, `mdadm:${name}`, name, `/dev/md/${name}`, md.content);
  }

  for (const [name, pool] of objectEntries(devices.zpool)) {
    const poolDevices = manifest.zfsMembers[name] || [];
    addObject(spec, "pools", name, {
      operation: "create",
      devices: poolDevices,
      mode: pool.mode,
      mountpoint: pool.mountpoint,
      properties: {
        ...(pool.options || {}),
        ...(pool.rootFsOptions || {}),
      },
      metadata: metadata(`zpool:${name}`, {
        postCreateHook: pool.postCreateHook,
      }),
    });
    for (const [datasetName, dataset] of objectEntries(pool.datasets)) {
      const fullName = `${name}/${datasetName}`;
      if (dataset.type === "zfs_volume") {
        addObject(spec, "zvols", fullName, {
          operation: "create",
          desiredSize: dataset.size,
          properties: dataset.options,
          metadata: metadata(`zpool:${name}`),
        });
        translateContent(spec, manifest, `zvol:${fullName}`, fullName, `/dev/zvol/${fullName}`, dataset.content);
      } else {
        addObject(spec, "datasets", fullName, {
          operation: "create",
          mountpoint: dataset.mountpoint,
          properties: dataset.options,
          metadata: metadata(`zpool:${name}`),
        });
      }
    }
  }

  for (const [mountpoint, nodev] of objectEntries(devices.nodev)) {
    addObject(spec, "filesystems", `nodev:${mountpoint}`, {
      operation: "mount",
      device: nodev.fsType,
      fsType: nodev.fsType,
      mountpoint,
      options: nodev.mountOptions,
      metadata: metadata(`nodev:${mountpoint}`),
    });
  }

  for (const [name, fs] of objectEntries(devices.bcachefs_filesystems)) {
    const existing = ensureCollection(spec, "filesystems")[`bcachefs:${name}`];
    if (existing) {
      existing.mountpoint = fs.mountpoint || existing.mountpoint;
      existing.metadata = {
        ...existing.metadata,
        passwordFile: fs.passwordFile,
        uuid: fs.uuid,
        subvolumes: fs.subvolumes,
        extraFormatArgs: fs.extraFormatArgs,
      };
    }
  }
}

async function main() {
  await mkdir(outDir, { recursive: true });
  const files = await findNixFiles(upstreamDir);
  const manifest = [];
  for (const file of files) {
    const sourcePath = path.join(upstreamDir, file);
    const sourceText = await readFile(sourcePath, "utf8");
    const evaluated = evalDiskoDevices(sourcePath, sourceText);
    const devices = evaluated.devices;
    const spec = {
      version: 1,
      apply: {
        allowDestructive: true,
        allowFormat: true,
        allowGrow: true,
        allowOffline: true,
        allowPotentialDataLoss: true,
        allowShrink: true,
        requireBackup: false,
      },
      metadata: { upstream: "nix-community/disko", source: file },
    };
    const item = {
      source: file,
      output: `${stableName(file)}.json`,
      originalDisks: [],
      testDisks: [],
      mdMembers: {},
      zfsMembers: {},
      unsupported: [],
      notes: [],
    };
    if (sourceText.includes("hybrid")) {
      item.notes.push(`${file}: hybrid MBR/GPT fields are preserved in partition metadata`);
    }
    item.notes.push(...evaluated.notes.map((note) => `${file}: ${note}`));
    translateTopLevel(spec, item, devices);
    delete item.mdMembers;
    delete item.zfsMembers;
    delete item.partitionState;
    delete item.coalescedPhysicalDisks;
    await writeFile(path.join(outDir, item.output), `${JSON.stringify(sorted(spec), null, 2)}\n`);
    manifest.push(item);
  }
  await writeFile(path.join(outDir, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`translated ${files.length} disko examples into ${outDir}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
