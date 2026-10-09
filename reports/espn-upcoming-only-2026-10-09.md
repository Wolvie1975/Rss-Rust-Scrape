# ESPN schedule cutoff — 2026-10-09

Both WNBA league and Kansas City Current NWSL imports now collect only today-or-later
fixtures using America/Chicago calendar dates, excluding completed games even on
today's date. Requests start in the current month and extend through next calendar
year, rather than refreshing historical seasons. Explicit season settings still
respect the cutoff. Past seasons produce no requests or observations.

Future cancellation/postponement observations remain available for stable-ID
reconciliation; the upcoming UI query excludes completed/canceled statuses and
historical rows. Existing DB rows are retained; no deletion or results-sync changes.
My-Newsfeed checkout is still absent, so its date/status query contract is documented
in both provider guides rather than implemented in that repository.

Live staging CLI dry-runs returned 3 Kansas City games (Oct 17, Oct 25, Nov 1) and
13 WNBA fixtures including current playoffs. ESPN's timeValid=false WNBA dates use
midnight Eastern placeholders: 2026-10-11T04:00Z is advertised as 10/11 TBD. The parser
now preserves that date instead of labeling it Oct 10 Central. Confirmed starts
retain UTC-to-Central conversion. Tests cover both summer/winter placeholders,
both leagues, inclusive today, past dates, completed games and empty past seasons.

All-target validation passed: 69 main-binary tests plus 19 example tests, with
seven DB integration tests opt-in. No production enabling/import/deployment or
scheduler restart was performed.
