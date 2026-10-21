# Portable Projects (NHLPROJECT)

A `.nhlproject` archive is a complete Studio project packed for another Studio user: the working database and dictionary, staged asset files, the change record, the project details and the project's inputs. It is how collaborators hand a mod back and forth. Players do not need it; they need the `.nhlmod` the project builds ([NHLMOD Packages](NHLMOD-Packages)).

## Creating one

In Project Overview press **Package Dev Project Files** ("Saves and packages your complete project for sharing."). The archive is written under your project storage and its path shown.

## Opening one

On the Modding Studio screen choose **Import Project Package** (or use the import control on **Load Project**). Studio extracts the archive into a new folder under your project storage and opens it. If the project is imported but fails to open, the message names where it was saved and offers **Retry opening project**.

## Relinking the game source

The archive does not contain the game. After importing, the project's **Game source folders** (Project Overview) point at the folders the original author used; use **Update sources** to point them at your own game dump and extracted assets. Database editing works without a linked game; asset previews and replacements need one.

## Projects with flexible inputs

A project may have been created from any one of a main database, a dictionary or a folder of extracted assets. Each kind works independently: a database-only project edits and packages without an installation; an assets-only project (the "Assets only" pill) stages texture edits, saves, packages without a database and deploys to its assets folder, while database routes are gated with a named reason; a roster-save project edits and packages for its own platform. The archive carries whichever inputs the project had.

## Preservation

Importing never deletes or rewrites your existing projects. Relocating a project folder is done from Project Overview's project-folder field so Studio's record of it stays correct.

Last reviewed: 2026-10-20
