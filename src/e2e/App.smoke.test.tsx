// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import previewCatalog from "../../public/dev-catalog.json";
import { App } from "../App";
import { useAppStore } from "../stores/app";

beforeEach(() => {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn().mockReturnValue({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }),
  });
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(
      new Response(JSON.stringify(previewCatalog), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    ),
  );
  useAppStore.setState({
    catalog: [],
    catalogLoading: true,
    catalogError: null,
    selectedToolId: null,
    toolPrefill: null,
    category: "home",
    paletteOpen: false,
    outputCenterOpen: false,
    settingsOpen: false,
    onboardingOpen: false,
    railCollapsed: false,
    persona: "freelancer",
    density: "comfortable",
    recentToolIds: [],
  });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("M0 shell smoke", () => {
  it("loads the registry shell and opens a fuzzy-matched tool by keyboard", async () => {
    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "Good morning." }),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("button", { name: /Export document/ }),
    ).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "k", metaKey: true });
    const palette = await screen.findByRole("dialog", {
      name: "Command palette",
    });
    expect(palette).toBeInTheDocument();

    const search = screen.getByPlaceholderText("What do you need?");
    fireEvent.change(search, {
      target: { value: "rszim" },
    });
    expect(
      screen.getByRole("option", { name: /Resize image/ }),
    ).toBeInTheDocument();

    fireEvent.keyDown(search, { key: "Enter" });
    expect(
      await screen.findByRole("heading", { name: "Resize image" }),
    ).toBeInTheDocument();
  });
});

describe("M8 onboarding and settings", () => {
  it("completes the three quiet onboarding steps with a selected profile", async () => {
    useAppStore.setState({ onboardingOpen: true });
    render(<App />);

    expect(
      await screen.findByRole("heading", {
        name: "Your work stays on this Mac.",
      }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("radio", { name: /Writer/ }));
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(
      screen.getByRole("heading", { name: "Pick your notes folder." }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(
      await screen.findByRole("heading", {
        name: "Add detail only when you want it.",
      }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Open Tooldeck" }));

    expect(useAppStore.getState().persona).toBe("writer");
    expect(useAppStore.getState().onboardingOpen).toBe(false);
  });

  it("opens settings and applies compact density immediately", async () => {
    render(<App />);
    await screen.findByRole("heading", { name: "Good morning." });
    fireEvent.click(screen.getByRole("button", { name: "Open settings" }));

    expect(
      await screen.findByRole("heading", { name: "Settings" }),
    ).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Density"), {
      target: { value: "compact" },
    });
    expect(document.documentElement.dataset.density).toBe("compact");
    fireEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(
      screen.getByRole("heading", { name: "Good morning." }),
    ).toBeInTheDocument();
  });
});
