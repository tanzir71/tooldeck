export type Persona = "freelancer" | "designer" | "writer" | "developer";
export type Density = "comfortable" | "compact";

const PROFILE_KEY = "tooldeck-profile";
const ONBOARDING_KEY = "tooldeck-onboarding-complete";
const DENSITY_KEY = "tooldeck-density";

function read(key: string): string | null {
  try {
    return window.localStorage?.getItem(key) ?? null;
  } catch {
    return null;
  }
}

function write(key: string, value: string) {
  try {
    window.localStorage?.setItem(key, value);
  } catch {
    // Preferences remain valid for this run when storage is unavailable.
  }
}

export function savedPersona(): Persona {
  const value = read(PROFILE_KEY);
  return value === "designer" || value === "writer" || value === "developer"
    ? value
    : "freelancer";
}

export function savePersona(persona: Persona) {
  write(PROFILE_KEY, persona);
}

export function savedDensity(): Density {
  return read(DENSITY_KEY) === "compact" ? "compact" : "comfortable";
}

export function saveDensity(density: Density) {
  write(DENSITY_KEY, density);
}

export function onboardingComplete(): boolean {
  return read(ONBOARDING_KEY) === "true";
}

export function saveOnboardingComplete(complete: boolean) {
  write(ONBOARDING_KEY, String(complete));
}
