export type ViewId = "downloads" | "library" | "player" | "settings";

interface SidebarProps {
  active: ViewId;
  onNavigate: (view: ViewId) => void;
}

const ITEMS: Array<{ id: ViewId; label: string }> = [
  { id: "downloads", label: "Downloads" },
  { id: "library", label: "Library" },
  { id: "player", label: "Player" },
  { id: "settings", label: "Settings" },
];

export function Sidebar({ active, onNavigate }: SidebarProps) {
  return (
    <nav className="sidebar" aria-label="Main navigation">
      <div className="sidebar-brand">Tide</div>
      <ul className="sidebar-nav">
        {ITEMS.map((item) => (
          <li key={item.id}>
            <button
              type="button"
              className={`sidebar-item${active === item.id ? " active" : ""}`}
              aria-current={active === item.id ? "page" : undefined}
              onClick={() => onNavigate(item.id)}
            >
              {item.label}
            </button>
          </li>
        ))}
      </ul>
      <div className="sidebar-footer">local-first · no cloud</div>
    </nav>
  );
}
