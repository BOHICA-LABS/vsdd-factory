//! Registration of this tool's own output paths and the pre-existing D-1149
//! `*-amendment-history.md` sidecar paths in
//! `plugins/vsdd-factory/config/artifact-path-registry.yaml`
//! (S-15.03 AC-006 delivery requirement; BC-10.13.001 §Architecture
//! Anchors).

use crate::error::MigrateError;
use std::path::Path;

/// Register this tool's own output paths (migration reports, rotation
/// archives) and the 5 pre-existing D-1149 `*-amendment-history.md` sidecar
/// paths into the artifact path registry at `registry_path`.
///
/// Read-only with respect to the 5 sidecar files themselves (BC-10.13.001
/// PC6) — this function only appends entries to the registry config; it
/// never opens a sidecar file for writing. Idempotent: re-running against an
/// already-registered set of paths must not add duplicate entries (each
/// sidecar's basename is checked against the current content before its
/// entry is appended).
pub fn register_artifact_paths(registry_path: &Path) -> Result<(), MigrateError> {
    let mut content =
        std::fs::read_to_string(registry_path).map_err(|source| MigrateError::Io {
            path: registry_path.to_path_buf(),
            source,
        })?;

    for rel in crate::migrate::TARGET_FILES {
        let rel_path = Path::new(rel);
        let stem = rel_path.file_stem().and_then(|s| s.to_str()).unwrap_or(rel);
        let basename = format!("{stem}-amendment-history.md");
        if content.contains(&basename) {
            // Idempotent: already registered from a prior run.
            continue;
        }
        let sidecar_dir = rel_path
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .filter(|p| !p.is_empty());
        let canonical_path_pattern = match sidecar_dir {
            Some(dir) => format!(".factory/{dir}/{basename}"),
            None => format!(".factory/{basename}"),
        };
        let artifact_type = format!("{}-amendment-history", stem.to_lowercase());
        let entry = format!(
            "\n  - artifact_type: {artifact_type}\n    canonical_path_pattern: \"{canonical_path_pattern}\"\n    description: Frozen pre-migration D-1149 amendment-history sidecar for {rel} (BC-10.13.001 PC6 — read-only; last-amended-migrate never mutates it)\n    enforcement_level: block\n"
        );
        content.push_str(&entry);
    }

    // S-15.03 SEC-003: write-then-rename, not a direct in-place write —
    // avoids a TOCTOU window where a concurrent reader/writer of the
    // registry could observe a partially-written file.
    crate::atomic_write::write_atomic(registry_path, &content).map_err(|e| {
        MigrateError::RegistryWrite {
            reason: format!("writing {}: {e}", registry_path.display()),
        }
    })?;
    Ok(())
}
