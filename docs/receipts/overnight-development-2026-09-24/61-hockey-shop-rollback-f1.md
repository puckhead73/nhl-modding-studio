# Receipt 61 — Hockey Shop: a failed undo now keeps the original file (coordinator finding F1)

**Developer-verified; not Codex-accepted; nothing launched; only owned scratch and disposable copies were written.** HEAD `8d60a6d`
(unchanged; nothing staged). Claim `dev-hockey-shop-rollback-f1`. Source: [receipt-60 review](../../validation/development-20260928-receipt60-review.md) F1.

## 1. Plain-language summary

- **Before:** if a Hockey Shop install failed part-way *and* putting a file back also failed, Studio still deleted the `.bak` backup
  holding that file's original. That could destroy the only copy of the original.
- **Now, when an undo cannot put a file back:**
  - Studio keeps the original on disk: the backup, or, when no backup was made, a new `<file>.nhlms-original` copy.
  - The error names the file that was not restored and where its original is.
  - Nothing Studio did not create is deleted or overwritten to force the undo. That covers a folder in the way, an older `.bak`,
    a file already named like Studio's temporary file, and a created file that someone changed since.
  - A clean-up that fails is reported separately. It is never reported as "everything restored".
- This is best-effort recovery during one install. It is **not** crash-safe and does not protect against another program writing
  the same files.

## 2. Change (`crates/studio-api/src/hockey_shop.rs`)

- **`WriteTxn` entries now record:** the prior bytes, the bytes written, the backup this transaction created, whether the destination
  was actually replaced, and any temporary file it could not remove.
- **Ownership:**
  - A backup or temporary file is written with `create_new`.
  - An existing `<file>.nhlms-tmp` refuses the write before anything is touched, including before a backup is made.
  - An existing `.bak` is never replaced or removed.
- **Rollback (reverse order):**
  - A destination that was never replaced needs nothing.
  - A replaced file is restored through a new temporary file and a rename, never truncated in place.
  - A created file is removed only while it still holds the bytes this install wrote.
  - A new backup is retired only after its destination is restored, and only while it still holds the original bytes.
  - On a failed restore the backup stays and is named. Without a backup, `<file>.nhlms-original` is written and named.
- The apply error now separates "restoring was incomplete: …" from "Not cleaned up: …".
- `backup_once` is removed; it is unused now that backups go through `create_new`.

## 3. Evidence

- **Unit tests, `studio-api --lib hockey_shop`: 11 / 11.** Seven fault cases:
  - `a_failed_restore_keeps_the_backup_and_names_it`: the coordinator's reproduction, with the assertion inverted. After an
    overwrite with backup, a folder replaces the file. Rollback reports the path and the `.bak`, the `.bak` still holds
    `original`, and the folder is left alone. The reviewed code deleted the `.bak` here (the coordinator's harness:
    `backup_survives=false`).
  - `a_failed_restore_without_a_backup_saves_a_recovery_copy`: `.nhlms-original` holds the original and is named.
  - `a_pre_existing_backup_survives`: an older `.bak` is untouched through the write and the rollback.
  - `a_failed_install_restores_what_it_wrote`: an ordinary rollback restores, retires the new backup and leaves no temporary files.
  - `created_files_are_removed_only_while_unchanged`: create-mode removal; a changed clone is kept and reported.
  - `an_unowned_temporary_name_is_never_overwritten`: the write refuses; the file, the destination and the rollback are untouched;
    no backup is made.
  - `a_failed_replacement_leaves_the_destination_and_no_temporary_file`: a read-only destination on Windows makes the rename fail.
    The temporary file is removed, the destination is unchanged, and the rollback retires the new backup.
- **Real NHL 12 regression:** `hockey_shop_title_spec` 2 / 2 (`--include-ignored`, `NHL_STUDIO_FIXTURES` set). The applied create
  and overwrite on a disposable copy of 44 files pass unchanged.

## 4. Limits

- Folders created for a new file are not removed on rollback (unchanged behaviour).
- No crash, power-loss or concurrent-writer protection is claimed.
