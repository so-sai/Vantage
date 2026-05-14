import { ArenaView } from './ArenaView';
import { useArena } from '../hooks/useArena';

export function App() {
  const { items, loading } = useArena();

  if (loading) return <div>Loading...</div>;

  return (
    <div className="app">
      <h1>Arena Dashboard</h1>
      <ArenaView items={items} />
    </div>
  );
}
