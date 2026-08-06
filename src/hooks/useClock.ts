import { useEffect, useState } from 'react';

/** Live wall-clock time for the idle clock face. Polled frequently enough for a
 * smooth second-hand sweep without being wasteful. */
export function useClock(): Date {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 200);
    return () => clearInterval(id);
  }, []);

  return now;
}
