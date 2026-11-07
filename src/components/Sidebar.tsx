import { getLocale } from "../lib/i18n";
import { useStrings } from "../lib/useStrings";

export type ViewId = "downloads" | "torrents" | "library" | "player" | "settings" | "spike";

interface SidebarProps {
  active: ViewId;
  onNavigate: (view: ViewId) => void;
}

type Item = {
  id: ViewId;
  key?: "downloads" | "torrents" | "library" | "player" | "settings";
  label?: string;
  icon: IconName;
  group: "workspace" | "system";
};

const ITEMS: Item[] = [
  { id: "downloads", key: "downloads", icon: "downloads", group: "workspace" },
  { id: "torrents", key: "torrents", icon: "torrents", group: "workspace" },
  { id: "library", key: "library", icon: "library", group: "workspace" },
  { id: "player", key: "player", icon: "player", group: "workspace" },
  { id: "settings", key: "settings", icon: "settings", group: "system" },
];

// M0.5 spike surface is a development prototype: never part of the
// shipped 1.0 navigation (release builds compile the backend spike
// commands to fail-closed stubs already).
const DEV_ITEMS: Item[] = [
  { id: "spike", label: "Spike (M0.5)", icon: "spike", group: "system" },
];

type IconName = "downloads" | "torrents" | "library" | "player" | "settings" | "spike";

const ICON_PATHS: Record<IconName, string> = {
  downloads: "M12 3v11m0 0 4-4m-4 4-4-4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2",
  torrents: "M12 3 3 8l9 5 9-5-9-5ZM3 12l9 5 9-5M3 16l9 5 9-5",
  library: "M4 5h6v14H4zM14 5h6v14h-6zM6 9h2M16 9h2",
  player: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18ZM10 8.5l5.5 3.5L10 15.5z",
  settings: "M4 7h9m4 0h3M4 17h3m4 0h9M13 5v4M7 15v4",
  spike: "M9 3h6M10 3v6L4.5 19A2 2 0 0 0 6.2 22h11.6a2 2 0 0 0 1.7-3L14 9V3M7.5 15h9",
};

function NavIcon({ name }: { name: IconName }) {
  return (
    <svg
      className="nav-icon"
      viewBox="0 0 24 24"
      width="18"
      height="18"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={ICON_PATHS[name]} />
    </svg>
  );
}

/** The Tide mark — also used by the first-launch welcome card. */
export function BrandMark({ size = 36 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 40 40" aria-hidden="true">
      <defs>
        <linearGradient id="tide-brand-gradient" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#4d8dff" />
          <stop offset="100%" stopColor="#35d0ba" />
        </linearGradient>
      </defs>
      <rect width="40" height="40" rx="12" fill="url(#tide-brand-gradient)" />
      <path
        d="M4 22c4-6 8-6 12 0s8 6 12 0 6-6 8-3"
        fill="none"
        stroke="#fff"
        strokeWidth="2.4"
        strokeLinecap="round"
      />
      <path
        d="M4 29c4-6 8-6 12 0s8 6 12 0 6-6 8-3"
        fill="none"
        stroke="#fff"
        strokeOpacity="0.55"
        strokeWidth="2.2"
        strokeLinecap="round"
      />
      <circle cx="28" cy="12" r="3" fill="#fff" fillOpacity="0.92" />
    </svg>
  );
}

export function Sidebar({ active, onNavigate }: SidebarProps) {
  const strings = useStrings();
  const zh = getLocale() === "zh-CN";
  const items = import.meta.env.DEV ? [...ITEMS, ...DEV_ITEMS] : ITEMS;
  const groups = [
    {
      key: "workspace",
      label: zh ? "工作区" : "Workspace",
      items: items.filter((i) => i.group === "workspace"),
    },
    {
      key: "system",
      label: zh ? "系统" : "System",
      items: items.filter((i) => i.group === "system"),
    },
  ];
  return (
    <nav className="sidebar" aria-label="Main navigation">
      <div className="sidebar-brand">
        <BrandMark size={36} />
        <span className="sidebar-brand-text">
          <strong>Tide</strong>
          <small>{zh ? "本地媒体，随心流动" : "local media, in flow"}</small>
        </span>
      </div>

      {groups.map((group) => (
        <div className="sidebar-group" key={group.key}>
          <div className="sidebar-group-title" aria-hidden="true">
            {group.label}
          </div>
          <ul className="sidebar-nav">
            {group.items.map((item) => (
              <li key={item.id}>
                <button
                  type="button"
                  className={`sidebar-item${active === item.id ? " active" : ""}`}
                  aria-current={active === item.id ? "page" : undefined}
                  onClick={() => onNavigate(item.id)}
                >
                  <NavIcon name={item.icon} />
                  <span className="sidebar-nav-label">
                    {item.key ? strings.sidebar[item.key] : (item.label ?? item.id)}
                  </span>
                  {active === item.id && <span className="sidebar-active-dot" aria-hidden="true" />}
                </button>
              </li>
            ))}
          </ul>
        </div>
      ))}

      <div className="sidebar-footer">
        <span className="footer-dot" aria-hidden="true" />
        <span className="sidebar-footer-text">{strings.sidebar.footer}</span>
      </div>
    </nav>
  );
}
