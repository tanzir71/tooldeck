import { useAppStore } from "../stores/app";
import { CommandPaletteDialog } from "./CommandPaletteDialog";

export function CommandPalette() {
  const open = useAppStore((state) => state.paletteOpen);
  return open ? <CommandPaletteDialog /> : null;
}
