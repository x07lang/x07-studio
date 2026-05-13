export interface VoiceCapture {
	start(): void;
	stop(): void;
	onPartial(callback: (text: string, confidence: number) => void): void;
	onFinal(callback: (text: string, confidence: number) => void): void;
	onMatch(words: string[], handler: () => void): void;
	supported: boolean;
}

type SpeechRecognitionCtor = new () => {
	lang: string;
	interimResults: boolean;
	continuous: boolean;
	onresult: ((event: SpeechRecognitionEventLike) => void) | null;
	onend: (() => void) | null;
	start(): void;
	stop(): void;
};

interface SpeechRecognitionEventLike {
	resultIndex: number;
	results: ArrayLike<{
		isFinal: boolean;
		[index: number]: { transcript: string; confidence: number };
	}>;
}

export function createVoiceCapture(language = 'en-US', continuous = false): VoiceCapture {
	const ctor = recognitionCtor();
	const partials: Array<(text: string, confidence: number) => void> = [];
	const finals: Array<(text: string, confidence: number) => void> = [];
	const matchers: Array<{ words: string[]; handler: () => void }> = [];
	if (!ctor) {
		return {
			supported: false,
			start: () => undefined,
			stop: () => undefined,
			onPartial: (cb) => partials.push(cb),
			onFinal: (cb) => finals.push(cb),
			onMatch: (words, handler) => matchers.push({ words, handler })
		};
	}
	const recognition = new ctor();
	recognition.lang = language;
	recognition.interimResults = true;
	recognition.continuous = continuous;
	recognition.onresult = (event) => {
		for (let index = event.resultIndex; index < event.results.length; index += 1) {
			const result = event.results[index];
			const item = result[0];
			const text = item.transcript.trim();
			const confidence = Number.isFinite(item.confidence) ? item.confidence : 0.75;
			notifyMatches(text, matchers);
			if (result.isFinal) finals.forEach((cb) => cb(text, confidence));
			else partials.forEach((cb) => cb(text, confidence));
		}
	};
	return {
		supported: true,
		start: () => recognition.start(),
		stop: () => recognition.stop(),
		onPartial: (cb) => partials.push(cb),
		onFinal: (cb) => finals.push(cb),
		onMatch: (words, handler) => matchers.push({ words, handler })
	};
}

function notifyMatches(
	text: string,
	matchers: Array<{ words: string[]; handler: () => void }>
) {
	const normalized = ` ${text.toLowerCase().replace(/\s+/g, ' ')} `;
	for (const matcher of matchers) {
		if (matcher.words.some((word) => normalized.includes(` ${word.toLowerCase()} `))) {
			matcher.handler();
		}
	}
}

function recognitionCtor(): SpeechRecognitionCtor | null {
	if (typeof window === 'undefined') return null;
	const candidate = (window as unknown as Record<string, unknown>).SpeechRecognition ??
		(window as unknown as Record<string, unknown>).webkitSpeechRecognition;
	return typeof candidate === 'function' ? (candidate as SpeechRecognitionCtor) : null;
}
