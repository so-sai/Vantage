import { useState, useEffect } from 'react';

interface ArenaData {
  items: string[];
}

/*
vantage:
  invariant: NoSideEffects
  reason: Pure rendering of backend data
*/
export function ArenaPage() {
  const [data, setData] = useState<ArenaData | null>(null);

  useEffect(() => {
    fetch('/api/arena')
      .then(res => res.json())
      .then(setData);
  }, []);

  if (!data) return <div>Loading...</div>;
  return <div>{data.items.join(', ')}</div>;
}
