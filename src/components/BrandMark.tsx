interface BrandMarkProps {
  compact?: boolean;
}

export function BrandMark({ compact = false }: BrandMarkProps) {
  return (
    <div className="brand" aria-label="Tooldeck">
      <span className="brand-mark" aria-hidden="true">
        <i />
        <i />
        <i />
      </span>
      {!compact && <span className="brand-name">Tooldeck</span>}
    </div>
  );
}
