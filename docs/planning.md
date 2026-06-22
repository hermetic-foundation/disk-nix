# Planning

`disk-nix plan` reads a desired storage JSON document and emits a
risk-classified action plan.

The command accepts either a direct spec:

```json
{
  "filesystems": {
    "root": {
      "mountpoint": "/",
      "fsType": "xfs",
      "resizePolicy": "grow-only",
      "preserveData": true
    }
  }
}
```

or the NixOS module wrapper written to `/etc/disk-nix/spec.json`:

```json
{
  "spec": {
    "filesystems": {}
  },
  "apply": {}
}
```

Current planning is intentionally conservative. It classifies filesystem
resize policy and preservation intent, then reports destructive or potentially
destructive behavior with alternatives.

Examples:

- `resizePolicy = "grow-only"` is classified as online growth intent.
- `resizePolicy = "shrink-allowed"` is classified as potential data loss and
  recommends migration or backup-first alternatives.
- `preserveData = false` is classified as destructive because it permits
  formatting or replacement.

Future planners should compare desired state against the probed topology before
emitting concrete executor actions.
