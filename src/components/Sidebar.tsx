export type ViewId = "downloads" | "torrents" | "library" | "player" | "settings" | "spike";

interface SidebarProps {
  active: ViewId;
  onNavigate: (view: ViewId) => void;
}

const ITEMS: Array<{ id: ViewId; label: string }> = [
  { id: "downloads", label: "Downloads" },
  { id: "torrents", label: "Torrents" },
  { id: "library", label: "Library" },
  { id: "player", label: "Player" },
  { id: "settings", label: "Settings" },
];

// M0.5 spike surface is a development prototype: never part of the
// shipped 1.0 navigation (release builds compile the backend spike
// commands to fail-closed stubs already).
const DEV_ITEMS: Array<{ id: ViewId; label: string }> = [
  { id: "spike", label: "Spike (M0.5)" },
];

export function Sidebar({ active, onNavigate }: SidebarProps) {
  const items =
    import.meta.env.DEV ? [...ITEMS, ...DEV_ITEMS] : ITEMS;
  return (
    <nav className="sidebar" aria-label="Main navigation">
      <div className="sidebar-brand">Tide</div>
      <ul className="sidebar-nav">
        {items.map((item) => (
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
