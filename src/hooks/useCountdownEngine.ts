import { useCallback, useEffect, useRef, useState } from 'react';

export type CountdownStatus = 'idle' | 'running' | 'paused' | 'finished';

export interface UseCountdownEngineOptions {
  onTick?: (remainingSeconds: number) => void;
  onMinuteBoundary?: (remainingSeconds: number) => void;
  onComplete?: () => void;
}

export interface CountdownEngine {
  status: CountdownStatus;
  remainingSeconds: number;
  start: (totalSeconds: number) => void;
  pause: () => void;
  resume: () => void;
  reset: () => void;
}

const POLL_MS = 200;

/** Drift-free countdown: always derives remaining time from a fixed end timestamp
 * (Date.now() diff) rather than accumulating interval ticks, so backgrounding /
 * slow frames never cause the displayed time to drift from real elapsed time. */
export function useCountdownEngine(options: UseCountdownEngineOptions = {}): CountdownEngine {
  const [status, setStatus] = useState<CountdownStatus>('idle');
  const [remainingSeconds, setRemainingSeconds] = useState(0);

  const endTimeRef = useRef<number | null>(null);
  const pausedRemainingRef = useRef(0);
  const lastEmittedSecondRef = useRef<number | null>(null);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  const start = useCallback((totalSeconds: number) => {
    if (totalSeconds <= 0) return;
    endTimeRef.current = Date.now() + totalSeconds * 1000;
    lastEmittedSecondRef.current = totalSeconds;
    setRemainingSeconds(totalSeconds);
    setStatus('running');
  }, []);

  const pause = useCallback(() => {
    setStatus((prev) => {
      if (prev !== 'running' || endTimeRef.current == null) return prev;
      const remaining = Math.max(0, Math.round((endTimeRef.current - Date.now()) / 1000));
      pausedRemainingRef.current = remaining;
      endTimeRef.current = null;
      return 'paused';
    });
  }, []);

  const resume = useCallback(() => {
    setStatus((prev) => {
      if (prev !== 'paused') return prev;
      endTimeRef.current = Date.now() + pausedRemainingRef.current * 1000;
      return 'running';
    });
  }, []);

  const reset = useCallback(() => {
    endTimeRef.current = null;
    pausedRemainingRef.current = 0;
    lastEmittedSecondRef.current = null;
    setRemainingSeconds(0);
    setStatus('idle');
  }, []);

  useEffect(() => {
    if (status !== 'running') return;

    const id = setInterval(() => {
      const endTime = endTimeRef.current;
      if (endTime == null) return;
      const remainingMs = endTime - Date.now();
      const remaining = Math.max(0, Math.round(remainingMs / 1000));
      setRemainingSeconds(remaining);

      if (lastEmittedSecondRef.current !== remaining) {
        const previous = lastEmittedSecondRef.current;
        lastEmittedSecondRef.current = remaining;
        if (remaining > 0) {
          optionsRef.current.onTick?.(remaining);
          if (previous != null && remaining % 60 === 0) {
            optionsRef.current.onMinuteBoundary?.(remaining);
          }
        }
      }

      if (remainingMs <= 0) {
        endTimeRef.current = null;
        setStatus('finished');
        optionsRef.current.onComplete?.();
      }
    }, POLL_MS);

    return () => clearInterval(id);
  }, [status]);

  return { status, remainingSeconds, start, pause, resume, reset };
}
