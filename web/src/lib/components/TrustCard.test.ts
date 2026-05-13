import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { TrustPosture } from '$lib/studio';
import TrustCard from './TrustCard.svelte';

const posture: TrustPosture = {
	schema_version: 'x07.studio.trust_posture@0.1.0',
	session_id: 'st-test',
	captured_at: '2026-05-13T00:00:00Z',
	trust_profile: 'local-preview',
	worlds: ['solve-pure'],
	capabilities: [],
	budgets: {
		prover_seconds_used: 2,
		prover_seconds_cap: 10
	},
	proof_coverage: {
		support_pct: 100,
		proved_pct: 87,
		proof_count: 3,
		assumptions_open: 0
	},
	proof_support_notes: [],
	deltas: [],
	posture_color: 'green'
};

describe('TrustCard', () => {
	it('renders the captured posture headline when posture exists', () => {
		render(TrustCard, { props: { posture } });

		expect(screen.getByText('solve-pure · 0 OS reads · 87% proof coverage')).toBeInTheDocument();
	});

	it('renders a computing state before posture is captured', () => {
		const { container } = render(TrustCard, { props: { posture: null, isComputing: true } });

		expect(screen.getByText('Computing trust posture...')).toBeInTheDocument();
		expect(screen.getByText("I'm working on it - checking the build, generating proofs, capturing posture.")).toBeInTheDocument();
		expect(container.querySelector('.trust-card.computing')).not.toBeNull();
	});

	it('renders the idle pending state when no session work is active', () => {
		const { container } = render(TrustCard, { props: { posture: null, isComputing: false } });

		expect(screen.getByText('Trust posture pending')).toBeInTheDocument();
		expect(screen.getByText('Build or formalize a session to capture the first posture.')).toBeInTheDocument();
		expect(container.querySelector('.trust-card.computing')).toBeNull();
	});
});
