# MS-02 — NHL 07 Xbox 360 acceptance, 2026-10-06

Status: PASSED — all 14 checks user-confirmed. The user confirmed ISO error handling and cancellation/retry both pass after correction. MS-02 is closed for NHL 07 Xbox 360. MS-01 remains passed.

| Check | User result |
| --- | --- |
| 1. Cancel navigation and pickers | PASS |
| 2. Validate dumped-folder selection | PASS |
| 3. Vanilla result and region | PASS |
| 4. Modified detection, acknowledgement and rescan | PASS |
| 5. Fresh projects use vanilla anchors | PASS |
| 6. Incomplete source | PASS |
| 7. ISO error handling | PASS after correction and user retest |
| 8. ISO cancellation and retry | PASS after correction and user retest |
| 9. Asset extraction reaches a stable completed state | PASS |
| 10. Asset cancellation, resume and conflicts | PASS |
| 11. Skip for now | PASS |
| 12. Import an existing database | PASS |
| 13. Optional inputs and asset-source import | PASS |
| 14. Missing-source detection and relinking | PASS |

## ISO correction

The API rejected every existing destination before inspecting the ISO. Cancelled extraction left partial files at that destination, blocking retry with the same misleading new-folder message.

Extraction now identifies readable ISO input before checking destination contents, accepts new or empty directories, and writes to a private sibling temporary directory. Only completed output is published to the selected destination. Cancellation/failure cleans this attempt's temporary files. Existing destination contents are preserved, including files introduced during extraction. The bundled Windows extract-xiso engine remains in use.

Legacy partial folders created before this fix are not automatically removed: their files cannot safely be treated as disposable. Use a new or empty destination for the first retest. Subsequent cancellation/retry can reuse that destination. Hard process termination may leave a private temporary folder; automatic crash cleanup is not claimed.

Automated verification: three domain extraction tests and three API job tests pass. Coverage includes invalid ISO errors ahead of destination collisions, empty destination success, cancellation and same-destination retry with absent/empty directories, destination conflicts, source preservation, ISO9660, and the bundled Xbox extraction engine. All 26 OpenProject frontend tests and TypeScript checking pass; terminal failure/cancellation tests also retry the same selection and create the project only after completion.

## Completed user retest

The user confirmed both checks below pass. No acceptance checks remain open for this target.

7. Select an invalid ISO using a new/empty destination: expect an ISO-related error. Correct the selection and retry. Separately use a valid ISO with a nonempty destination: expect an existing-files explanation with no overwrites.
8. Start valid ISO extraction to a new/empty destination, cancel during extraction, wait for it to stop, then click Next again without changing the destination. Expect successful extraction and onboarding completion.

These results qualify NHL 07 Xbox 360 only. Other games/platforms and additional region references require their own acceptance.
