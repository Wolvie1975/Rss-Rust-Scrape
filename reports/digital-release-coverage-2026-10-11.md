# Digital release coverage correction — 2026-10-11

The user selected the current Central Monday–Sunday week with improved VOD/digital
coverage, using Movie Insider's 2026 digital list as the reference.

Read-only production audit found 33 movies/33 source links/76 release records.
The existing digital source was DVDs Release Dates; its coverage included older
catalog/holiday films that did not appear in the reference week's Movie Insider
list. This difference alone does not establish that those availability dates are
wrong. The When To Stream parser had a definite gap: it ignored explicit VOD dates.

Implemented:

- Movie Insider digital provider using the accessible public VOD calendar, all
  pagination, current-week candidate selection and explicit detail-page digital
  dates. Calendar cards can be out of order; no date-based early stopping.
- Numeric movie IDs and digital:first release keys preserve identity across date
  and slug changes. Announced digital dates stay separate from theaters/discs/SVOD.
- Existing poster validation accepts the movie's own labeled poster, with source
  identity checks. No original-year or external-ID guesses from release dates.
- --configure-movieinsider / sql/movieinsider_seed.sql register disabled, preserving
  existing configuration. Existing --movies imports enabled providers.
- When To Stream explicit VOD dates now import as digital, alongside existing
  separate subscription dates and services.
- Source-specific preview flags, source error tracking and frontend handoff.

Live preview for October 5–11:

| Provider | Digital observations | Subscription observations | Issues |
| --- | ---: | ---: | ---: |
| Movie Insider | 10 | 0 | 0 |
| When To Stream | 5 | 9 | 0 |

These are provider observations, with overlap; they must not be summed as unique
movies. Both previews were read-only. Existing production rows were retained.

Validation: 113 offline tests pass; isolated SQL test checks disabled/enabled
settings, repeat imports, reschedules, changed slugs and preserved IDs/FirstSeenAt.
Clippy with warnings denied and formatting checks pass.

Production deployment, source enabling and My-Newsfeed frontend changes remain
pending. No new movie schema columns/tables or runtime privileges are required.
The frontend handoff recommends Movie Insider as the preferred digital source,
with explicit provider selection rather than silently combining catalog and
premiere observations. See docs/my-newsfeed-digital-releases.md.
