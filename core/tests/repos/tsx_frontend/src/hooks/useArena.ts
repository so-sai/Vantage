import { useState, useEffect } from 'react';

interface ArenaState {
  items: string[];
  loading: boolean;
}

/*
vantage:
  invariant: Idempotent
  reason: Hook must be safe for React strict mode double-invoke
*/
export function useArena(): ArenaState {
  const [items, setItems] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch('/api/arena')
      .then(res => res.json())
      .then(data => {
        setItems(data.items);
        setLoading(false);
      });
  }, []);

  return { items, loading };
}
