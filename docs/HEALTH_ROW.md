# Health row

HealthRow is the early canonical-loop status strip above the TrustCard.

## Pills

- Doctor: `x07 doctor`
- Lockfile: `x07 pkg lock --project x07.json --check`
- Migrate: `x07 migrate --check --to 0.5` and
  `x07 project migrate --check --project x07.json`

Overall color is green when all checks are clean, amber when warnings, stale
lockfile, or migration work is present, and red when doctor or lockfile blockers
are present. The migration action takes a `.x07/studio/migrate-backup-*` copy of
`x07.json` and `x07.lock.json` before running write-mode migrations.
