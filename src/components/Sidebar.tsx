import { useStrings } from "../lib/useStrings";

export type ViewId = "downloads" | "torrents" | "library" | "player" | "settings" | "spike";

interface SidebarProps {
  active: ViewId;
  onNavigate: (view: ViewId) => void;
}

type Item = { id: ViewId; key?: "downloads" | "torrents" | "library" | "player" | "settings"; label?: string };

const ITEMS: Item[] = [
  { id: "downloads", key: "downloads" },
  { id: "torrents", key: "torrents" },
  { id: "library", key: "library" },
  { id: "player", key: "player" },
  { id: "settings", key: "settings" },
];

// M0.5 spike surface is a development prototype: never part of the
// shipped 1.0 navigation (release builds compile the backend spike
// commands to fail-closed stubs already).
const DEV_ITEMS: Item[] = [
  { id: "spike", label: "Spike (M0.5)" },
];

export function Sidebar({ active, onNavigate }: SidebarProps) {
  const strings = useStrings();
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
              {item.key ? strings.sidebar[item.key] : (item.label ?? item.id)}
            </button>
          </li>
        ))}
      </ul>
      <div className="sidebar-footer">{strings.sidebar.footer}</div>
    </nav>
  );
}
