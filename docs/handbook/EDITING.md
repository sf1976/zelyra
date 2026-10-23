# Editing the Zelyra handbooks

The handbook has one canonical source in this repository. The German and
English editions are kept side by side in `docs/handbook/de/` and
`docs/handbook/en/`. `manifest.json` is the authoritative reading order for
each edition.

## Where to edit

- `README.md` in each language folder is the short, guided tutorial.
- `handbuch.md` / `handbook.md` is the full-book introduction, version status,
  release summary, and table of contents.
- `lehrbuch-grundlagen.md` / `textbook-foundations.md` contains the first
  textbook part.
- `lehrbuch-anwendungen.md` / `textbook-applications.md` contains application
  chapters, including the Studio chapter.
- `technisches-referenzhandbuch.md` / `technical-reference.md` is the technical
  reference.
- `anhaenge.md` / `appendices.md` contains the appendices.

Edit the relevant component rather than copying its content into another
handbook file. Keep equivalent German and English sections in sync, and update
the full-book introduction and release summary when the supported product
version changes. Add or reorder a component only by changing
`manifest.json`; the website uses that manifest to build its reading view and
downloadable editions.

The root `README.md` and `README.de.md` remain short project entry points. They
link to the handbook rather than duplicating it. Release notes remain separate
records of changes for one release; they are not handbook chapters.

## Version and accuracy

Describe the latest published stable release as available. Label work after
that release as planned, experimental, or unreleased. Check feature limits
against `docs/implemented.de.md` / `docs/implemented.en.md`, the relevant
reference sections, and the release notes before documenting a capability.
