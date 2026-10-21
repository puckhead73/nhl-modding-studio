# Contributing to the Wiki

This wiki is written as Markdown files under `wiki/` in the main repository and mirrored to GitHub's wiki pages by a workflow. Nothing is edited on the wiki site directly: a change there would be overwritten by the next sync. To change a page, edit its file in the repository and open a pull request.

## How pages are published

1. Every page is one file, `wiki/Page-Name.md`. Folders are not allowed because GitHub flattens them; `wiki/images/` is the one exception and holds screenshots.
2. A lint (`scripts/wiki_check.py`) runs in continuous integration on every push and pull request. It fails the build on broken links, pages missing from the sidebar, missing metadata, and any content that must not be public (see below).
3. On every push to `main` that touches `wiki/`, the *Wiki* workflow runs the lint again and pushes the folder to the wiki repository. Maintainers can also run it by hand with a dry-run option.
4. Some pages are **generated** from the project's audit data by `scripts/wiki_generate.py`: the per-title schema pages, the schema index, the save-format table and the surface-capability matrix. They begin with a banner saying so and must not be edited by hand; continuous integration refuses a tree where they are out of date. To refresh them after the audit data changes, run the script with `--write`, bump its review date, and commit the result.

## Writing conventions

**Plain language first.** Start each page with two to four sentences a newcomer can follow. Depth goes below, under headings, or in a collapsed block.

**Name things the way the app does.** Quote screen labels exactly as they appear, and write navigation paths with chevrons: `Game Database › Teams › Uniforms`. If a label changes in the app, the page changes with it.

**Readable names over raw identifiers.** User-guide pages talk about *teams*, *players*, *jersey numbers* and field names as the app shows them. Raw database table and field codes belong in a collapsed `<details>` block, never in the main text. Knowledge-base pages may lead with raw codes because the codes are the subject, but always put the readable meaning beside them.

**One sentence, one idea.** Short paragraphs; lists for parallel items; tables for anything with more than two columns of facts.

**Links.** Link to other pages by page name without a suffix: `[Core Concepts](Core-Concepts)`. Link to files in the repository with full `https://github.com/...` URLs. Images are referenced as `images/name.png`.

**Metadata.** The last line of every page is `Last reviewed: YYYY-MM-DD`, updated whenever the page is checked against the app or the evidence.

## Evidence words

The Studio project separates *what is implemented* from *what is proven*. The wiki uses the same vocabulary in plain words:

| Word | Meaning |
|---|---|
| **implemented** | The code exists. |
| **automated-tested** | A named automated test executed the operation against reference data and asserted the result. |
| **game-verified** | Someone ran the edited output on the exact title, platform and build and recorded what happened. **No title has reached this level yet.** |
| **blocked** | Implementation or evidence is blocked for a recorded reason. |
| **missing-fixture** | No sample data is available to test against. |

Knowledge-base pages carry an `**Evidence:**` line near the top and label individual claims as:

- **measured** — reproduced on a reference database or a named sample from a specific title, platform and build;
- **inferred** — follows from measured facts but has not itself been reproduced;
- **open** — unknown, under investigation, or contradicted by later work.

Writing a page never promotes a feature's status. If you are not sure which word applies, use the weaker one.

## What never goes on the wiki

The lint refuses these, and so should you:

- paths on anyone's machine (drive letters, private sample folders, emulator install folders);
- hashes of real game files;
- references to other tool repositories or their source;
- anything about the games' executables beyond what a user observes;
- unreleased titles and platforms;
- personal data about anyone.

Build identifiers that the games themselves expose (disc product codes such as `BLUS31179` or `CUSA18182`, and version strings such as `APP_VER 01.60`) are fine.

## Terminology

- The advanced mode of the application is called **Developer Mode**, never "advanced mode".
- The three tools on the start screen are **Mod Manager**, **Modding Studio** and **Save Editor**.
- A **Project** is the folder that holds your edits; a **Build** produces a `.nhlmod` package; the Mod Manager **activates** a package on a game installation.
- The status wording is: *beta, not released; no title is game-verified*.

Last reviewed: 2026-10-20
