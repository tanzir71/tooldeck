interface ShortcutProps {
  children: string;
}

export function Shortcut({ children }: ShortcutProps) {
  return <kbd className="shortcut">{children}</kbd>;
}
