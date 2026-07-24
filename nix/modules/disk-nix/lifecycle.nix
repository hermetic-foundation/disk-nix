{ lib, cleanSpecAttrs }:

let
  destroyOperations = [
    "destroy"
    "close"
    "deactivate"
    "logout"
    "unmount"
    "unexport"
    "detach"
    "stop"
    "remove-key"
    "remove-token"
  ];

  isDestroyLifecycle =
    object:
    let
      requestedOperation =
        if (object.operation or null) != null then object.operation else (object.action or null);
    in
    (object.destroy or false) || builtins.elem requestedOperation destroyOperations;

  isExportLifecycle =
    object:
    let
      requestedOperation =
        if (object.operation or null) != null then object.operation else (object.action or null);
    in
    requestedOperation == "export";

  activeLifecycleAttrs = attrs: lib.filterAttrs (_: object: !isDestroyLifecycle object) attrs;

  lifecyclePathTarget =
    name: object:
    if object.target != null then
      object.target
    else if object.path != null then
      object.path
    else if object.device != null then
      object.device
    else if lib.hasPrefix "/" name then
      name
    else
      null;

  lifecycleIdentity =
    name: object:
    if object.target != null then
      object.target
    else if object.path != null then
      object.path
    else
      name;

  lifecycleOperation =
    object:
    if (object.operation or null) != null then
      object.operation
    else if (object.action or null) != null then
      object.action
    else
      "create";

  lifecycleDesiredSize =
    object:
    if (object.desiredSize or null) != null then
      object.desiredSize
    else if (object.targetSize or null) != null then
      object.targetSize
    else
      object.size or null;

  lifecycleManagedEntry =
    identity: object:
    cleanSpecAttrs (
      {
        inherit identity;
        operation = lifecycleOperation object;
        desiredSize = lifecycleDesiredSize object;
      }
      // lib.optionalAttrs ((object.target or null) != null) {
        inherit (object) target;
      }
      // lib.optionalAttrs ((object.path or null) != null) {
        inherit (object) path;
      }
      // lib.optionalAttrs ((object.device or null) != null) {
        inherit (object) device;
      }
      // lib.optionalAttrs ((object.mountpoint or null) != null) {
        inherit (object) mountpoint;
      }
      // lib.optionalAttrs ((object.source or null) != null) {
        inherit (object) source;
      }
      // lib.optionalAttrs ((object.client or null) != null) {
        inherit (object) client;
      }
      // lib.optionalAttrs ((object.portal or null) != null) {
        inherit (object) portal;
      }
    );

  lifecycleManagedMap =
    attrs: identityFn:
    builtins.listToAttrs (
      lib.filter (entry: entry != null) (
        lib.mapAttrsToList (
          name: object:
          let
            identity = identityFn name object;
          in
          if identity == null then
            null
          else
            {
              name = identity;
              value = lifecycleManagedEntry identity object;
            }
        ) attrs
      )
    );
in
{
  inherit
    activeLifecycleAttrs
    isDestroyLifecycle
    isExportLifecycle
    lifecycleDesiredSize
    lifecycleIdentity
    lifecycleManagedEntry
    lifecycleManagedMap
    lifecycleOperation
    lifecyclePathTarget
    ;
}
