interface EmptyStateProps {
  title: string;
  hint?: string;
}

export function EmptyState({ title, hint }: EmptyStateProps) {
  return (
    <div className="empty-state">
      <div className="empty-art" aria-hidden="true">
        <svg viewBox="0 0 132 96" width="132" height="96" fill="none">
          <defs>
            <linearGradient id="tide-empty-gradient" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0%" stopColor="#4d8dff" />
              <stop offset="100%" stopColor="#35d0ba" />
            </linearGradient>
          </defs>
          <rect x="16" y="14" width="100" height="68" rx="14" fill="rgba(255,255,255,0.025)" stroke="rgba(255,255,255,0.11)" />
          <circle cx="98" cy="32" r="7" fill="url(#tide-empty-gradient)" opacity="0.85" />
          <path d="M16 58c12-14 24-14 36 0s24 14 36 0 20-12 28-5" stroke="url(#tide-empty-gradient)" strokeWidth="3" strokeLinecap="round" />
          <path d="M16 70c12-14 24-14 36 0s24 14 36 0 20-12 28-5" stroke="rgba(255,255,255,0.16)" strokeWidth="2.5" strokeLinecap="round" />
          <circle cx="34" cy="34" r="3" fill="rgba(255,255,255,0.22)" />
          <circle cx="46" cy="34" r="3" fill="rgba(255,255,255,0.14)" />
        </svg>
      </div>
      <p className="empty-title">{title}</p>
      {hint && <p className="empty-hint">{hint}</p>}
    </div>
  );
}
