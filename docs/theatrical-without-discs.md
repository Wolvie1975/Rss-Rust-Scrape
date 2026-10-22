# Theatrical releases without physical discs

October 22, 2026 (America/Chicago). Provider identity is independent of release
channel: DVDs Release Dates supplies theatrical and digital dates as well as discs.
Disabling its provider-wide Enabled flag hides all of those dates from the frontend.

## Configuration and public contract

`MovieReleaseSources.CollectDisc` is a new non-null BIT, default 1 to preserve
existing behavior. Apply the additive, repeatable `sql/movie_releases.sql` migration
with migration credentials before running this version. Runtime imports perform no DDL.
Provider `Enabled` remains the overall ingestion/public visibility gate.

Production DVDs Release Dates (source 5) uses `CollectDisc=0`. The explicit,
repeatable `sql/movie_disable_discs.sql` sets that preference and marks only that
provider's scheduled `disc` rows `withdrawn`. It does not enable the provider.
After successful live date verification and deployment, enable the existing source.
Other providers' enabled flags and disc preferences are preserved.

The collector skips this provider's disc calendars and excludes disc rows from
both calendar-discovered and tracked detail pages. Tracking excludes disc-only
movies when discs are disabled. The SQL upsert independently rejects disc writes
for CollectDisc=0, including attempts to update an existing release identity.
DVD, Blu-ray, UHD/4K and physical bundles that include Digital all map to `disc`;
standalone Digital HD remains `digital`. Theatrical, digital purchase/rental and
subscription mappings are unchanged. Existing enabled Movie Insider is digital-only
in this integration; When To Stream supplies VOD/subscription dates. Neither
currently replaces this provider's theatrical coverage.

No new frontend eligibility field is required: existing readers already require
provider Enabled and ReleaseStatus=scheduled. Withdrawn discs therefore stay
hidden in All releases as well as filtered views. Keep these existing checks.
Source, movie, source-link and release IDs are preserved; the existing stable
release key continues to update dates in place. Normal date retention still applies.
A missing/failed detail request does not withdraw unrelated releases or clear
last-good dates. CollectDisc is a collection preference, not a claim that the
publisher cancelled a release; the withdrawn status records local eligibility.

If discs are deliberately enabled later, restoring old withdrawn rows is a separate
explicit decision after fresh provider verification; this change does not automatically
reactivate locally withdrawn records. Before rolling back to an older image that
ignores CollectDisc, disable DVDs Release Dates again to keep discs excluded.
Do not restore the shared database or re-enable discs as a rollback shortcut.

## Live provider verification

An unrestricted read-only preview returned six theatrical, seven digital and 23
disc entries for the selected week. The policy preview returned six theatrical
and seven digital entries, no discs, no disc calendar, and zero source issues.
Each theatrical date below was parsed from its current provider detail page,
not copied from a calendar heading or inferred from our prior stored dates.

| Title | Confirmed provider US theatrical date | External movie ID |
| --- | --- | --- |
| A Talent for Murder | 2026-10-23 | 12904 |
| Ali G: Who Iz I? | 2026-10-23 | 12903 |
| Clayface | 2026-10-23 | 12534 |
| Klara and the Sun | 2026-10-23 | 12824 |
| Wildwood | 2026-10-23 | 12778 |
| You Can See Everything | 2026-10-23 | 12929 |

These are provider claims, not independent confirmation of every cinema booking.
All six matched the retained records; none required a date correction or exclusion.
The provider calendar is discovery only; detail dates remain authoritative for
this integration. For a policy preview without database writes:

```sh
cargo run --locked --example movie_preview -- --week-of 2026-10-23 --source dvdsreleasedates --exclude-discs
```

Production `--movies` reads provider policies from SQL automatically. The hourly
command and credentials require no new flag or environment variable.

## Validation

The Rust suite passed 163 tests, with 22 opt-in tests ignored. Two additional SQL
integration tests passed serially with migration credentials and rolled-back
fixtures: disc-write rejection, retained-disc tracking exclusion, repeat identity
upserts, all five non-disc channels, source disablement and date rescheduling.
The tests connect without running migrations. Runtime credentials intentionally
cannot change source Enabled; admin fixture setup uses the migration profile.
Strict Clippy passed. The migration and explicit disc policy each applied twice.
A fresh COPY_ONLY full database backup passed RESTORE VERIFYONLY before changes.
Before activation, 24 retained scheduled physical-disc rows were marked withdrawn;
no unrelated source rows were withdrawn or manually deleted.

See the [deployment report](../reports/theatrical-no-disc-20261022.md) for final runtime, repeat-import and browser evidence.
