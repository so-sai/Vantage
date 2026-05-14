/*
vantage:
  invariant: NoSideEffects
  reason: Prevent render cascade loops
*/
export function ArenaView({ items }: { items: string[] }) {
  return (
    <div className="arena">
      {items.map((item, i) => (
        <div key={i} className="arena-item">{item}</div>
      ))}
    </div>
  );
}
