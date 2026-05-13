import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { HealthSnapshot } from '$lib/studio';
import HealthRow from './HealthRow.svelte';

function healthSnapshot(from_schema: string | null, needs_migration = true): HealthSnapshot {
	return {
		schema_version: 'x07.studio.health_snapshot@0.1.0',
		captured_at: '2026-05-13T00:00:00Z',
		doctor: { ok: true, blockers: [], warnings: [] },
		lockfile: { ok: true, stale: false, yanked: [], advisories: [] },
		migrate: {
			needs_migration,
			from_schema,
			to_schema: '0.5',
			project_schema_legacy: false
		},
		subscriber_count: 0,
		active_sessions: 0,
		overall_color: 'green'
	};
}

describe('HealthRow', () => {
	it('hides the migration pill when no migration is needed', () => {
		render(HealthRow, { props: { snapshot: healthSnapshot(null, false) } });

		expect(screen.queryByText('Migrate')).not.toBeInTheDocument();
	});

	it('labels a fresh workspace migration as init', () => {
		render(HealthRow, { props: { snapshot: healthSnapshot(null) } });

		expect(screen.getByText('init → 0.5')).toBeInTheDocument();
	});

	it('labels existing workspace migrations with both schema versions', () => {
		render(HealthRow, { props: { snapshot: healthSnapshot('0.4') } });

		expect(screen.getByText('0.4 → 0.5')).toBeInTheDocument();
	});
});
