import { afterEach, describe, expect, it, vi } from 'vitest';

import { createVoiceCapture } from './voice';

class FakeRecognition {
	lang = '';
	interimResults = false;
	continuous = true;
	onresult: ((event: {
		resultIndex: number;
		results: ArrayLike<{
			isFinal: boolean;
			[index: number]: { transcript: string; confidence: number };
		}>;
	}) => void) | null = null;
	onend: (() => void) | null = null;
	start = vi.fn();
	stop = vi.fn();
}

let lastRecognition: FakeRecognition | null = null;

afterEach(() => {
	delete (window as unknown as Record<string, unknown>).SpeechRecognition;
	delete (window as unknown as Record<string, unknown>).webkitSpeechRecognition;
	lastRecognition = null;
});

describe('createVoiceCapture', () => {
	it('reports unsupported when Web Speech is absent', () => {
		const capture = createVoiceCapture();

		expect(capture.supported).toBe(false);
		expect(() => capture.start()).not.toThrow();
	});

	it('emits partial and final transcripts from Web Speech results', () => {
		const ctor = vi.fn(() => {
			lastRecognition = new FakeRecognition();
			return lastRecognition;
		});
		Object.defineProperty(window, 'SpeechRecognition', {
			value: ctor,
			configurable: true
		});

		const capture = createVoiceCapture('en-GB');
		const partial = vi.fn();
		const final = vi.fn();
		capture.onPartial(partial);
		capture.onFinal(final);
		capture.start();

		expect(capture.supported).toBe(true);
		expect(lastRecognition?.lang).toBe('en-GB');
		expect(lastRecognition?.interimResults).toBe(true);
		expect(lastRecognition?.continuous).toBe(false);
		expect(lastRecognition?.start).toHaveBeenCalledOnce();

		lastRecognition?.onresult?.({
			resultIndex: 0,
			results: [
				{ isFinal: false, 0: { transcript: ' draft text ', confidence: 0.42 } },
				{ isFinal: true, 0: { transcript: ' final text ', confidence: Number.NaN } }
			]
		});

		expect(partial).toHaveBeenCalledWith('draft text', 0.42);
		expect(final).toHaveBeenCalledWith('final text', 0.75);
	});
});
