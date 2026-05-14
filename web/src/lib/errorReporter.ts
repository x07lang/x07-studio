import type { TelemetryErrorReport } from './studio';

const TELEMETRY_KEY = 'x07-studio.telemetry.enabled';
const MAX_TEXT = 2048;

let installed = false;

export function telemetryEnabled() {
	return storage()?.getItem(TELEMETRY_KEY) === 'true';
}

export function setTelemetryEnabled(enabled: boolean) {
	const local = storage();
	if (!local) return;
	if (enabled) {
		local.setItem(TELEMETRY_KEY, 'true');
	} else {
		local.removeItem(TELEMETRY_KEY);
	}
}

export function installErrorReporter() {
	if (installed || typeof window === 'undefined') return;
	installed = true;
	window.addEventListener('error', (event) => {
		void reportClientError({
			consent: telemetryEnabled(),
			source: 'web',
			severity: 'error',
			message: event.message || 'Unhandled browser error',
			stack: event.error instanceof Error ? event.error.stack ?? null : null,
			route: window.location.pathname,
			user_agent: navigator.userAgent,
			context: { kind: 'window.error', filename: event.filename, line: event.lineno, column: event.colno },
			occurred_at: new Date().toISOString()
		});
	});
	window.addEventListener('unhandledrejection', (event) => {
		const reason = event.reason instanceof Error ? event.reason : null;
		void reportClientError({
			consent: telemetryEnabled(),
			source: 'web',
			severity: 'error',
			message: reason?.message ?? String(event.reason ?? 'Unhandled promise rejection'),
			stack: reason?.stack ?? null,
			route: window.location.pathname,
			user_agent: navigator.userAgent,
			context: { kind: 'unhandledrejection' },
			occurred_at: new Date().toISOString()
		});
	});
}

export async function reportFetchFailure(path: string, status: number, message: string) {
	if (typeof window === 'undefined' || path === '/v1/telemetry/error') return;
	await reportClientError({
		consent: telemetryEnabled(),
		source: 'web',
		severity: status >= 500 ? 'error' : 'warning',
		message: `HTTP ${status}`,
		route: window.location.pathname,
		user_agent: navigator.userAgent,
		context: { kind: 'fetch', path: redactPath(path), status, message: redactText(message) },
		occurred_at: new Date().toISOString()
	});
}

export async function reportClientError(report: TelemetryErrorReport) {
	if (!report.consent || typeof fetch === 'undefined') return;
	const payload: TelemetryErrorReport = {
		...report,
		message: redactText(report.message),
		stack: report.stack ? redactText(report.stack) : null,
		route: report.route ? redactPath(report.route) : null,
		context: redactContext(report.context)
	};
	await fetch('/v1/telemetry/error', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(payload)
	}).catch(() => undefined);
}

function redactPath(path: string) {
	return redactText(path.replace(/[0-9a-f]{8}-[0-9a-f-]{27,}/gi, ':uuid'));
}

function redactText(text: string) {
	return text
		.replace(/([A-Za-z]:)?\/[^\s"']+/g, ':path')
		.replace(/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g, ':email')
		.slice(0, MAX_TEXT);
}

function redactContext(value: unknown): unknown {
	if (typeof value === 'string') return redactText(value);
	if (Array.isArray(value)) return value.slice(0, 16).map(redactContext);
	if (value && typeof value === 'object') {
		return Object.fromEntries(
			Object.entries(value)
				.slice(0, 16)
				.map(([key, item]) => [redactText(key), redactContext(item)])
		);
	}
	return value;
}

function storage(): Storage | null {
	if (typeof window === 'undefined') return null;
	const local = window.localStorage;
	if (!local || typeof local.getItem !== 'function') return null;
	return local;
}
