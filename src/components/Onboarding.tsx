import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState } from "react";

import { getNotesVault, setNotesVault } from "../lib/notes";
import type { Persona } from "../lib/preferences";
import { useAppStore } from "../stores/app";
import { ActivityPermissionPanel } from "./ActivityPermissionPanel";
import { BrandMark } from "./BrandMark";
import { Icon } from "./Icon";

const personas: {
  id: Persona;
  name: string;
  detail: string;
  icon: string;
}[] = [
  {
    id: "freelancer",
    name: "Freelancer",
    detail: "Time, documents, notes, and PDF",
    icon: "briefcase",
  },
  {
    id: "designer",
    name: "Designer",
    detail: "Images, capture, media, and notes",
    icon: "image",
  },
  {
    id: "writer",
    name: "Writer",
    detail: "Notes, documents, and PDF",
    icon: "square-pen",
  },
  {
    id: "developer",
    name: "Developer",
    detail: "Files, notes, capture, and utilities",
    icon: "code",
  },
];

export function Onboarding() {
  const finishOnboarding = useAppStore((state) => state.finishOnboarding);
  const savedProfile = useAppStore((state) => state.persona);
  const [step, setStep] = useState(0);
  const [persona, setPersona] = useState<Persona>(savedProfile);
  const [vault, setVault] = useState("Loading local vault…");
  const [vaultError, setVaultError] = useState<string | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    void getNotesVault().then(setVault);
  }, []);

  useEffect(() => {
    headingRef.current?.focus();
  }, [step]);

  async function chooseVault() {
    setVaultError(null);
    try {
      const selected = isTauri()
        ? await open({ directory: true, multiple: false })
        : "/Users/you/Documents/Tooldeck Notes";
      if (typeof selected === "string") {
        setVault(await setNotesVault(selected));
      }
    } catch (error) {
      setVaultError(
        error instanceof Error
          ? error.message
          : "That folder could not be used.",
      );
    }
  }

  function finish() {
    finishOnboarding(persona);
  }

  return (
    <div className="onboarding-scrim">
      <section
        className="onboarding"
        role="dialog"
        aria-modal="true"
        aria-labelledby="onboarding-title"
      >
        <header className="onboarding-top">
          <BrandMark />
          <span>Step {step + 1} of 3</span>
        </header>
        <div className="onboarding-progress" aria-hidden="true">
          {[0, 1, 2].map((item) => (
            <span className={item <= step ? "active" : ""} key={item} />
          ))}
        </div>

        <div className="onboarding-body">
          {step === 0 && (
            <>
              <span className="onboarding-hero-icon">
                <Icon name="shield" size={23} />
              </span>
              <p className="eyebrow">Private by design</p>
              <h1 id="onboarding-title" ref={headingRef} tabIndex={-1}>
                Your work stays on this Mac.
              </h1>
              <p className="onboarding-copy">
                Choose a profile to put the tools you use most within one click.
                Every tool remains searchable.
              </p>
              <div
                className="persona-grid"
                role="radiogroup"
                aria-label="Profile"
              >
                {personas.map((item) => (
                  <button
                    type="button"
                    role="radio"
                    aria-checked={persona === item.id}
                    className={`persona-card ${
                      persona === item.id ? "selected" : ""
                    }`}
                    key={item.id}
                    onClick={() => setPersona(item.id)}
                  >
                    <Icon name={item.icon} size={17} />
                    <span>
                      <strong>{item.name}</strong>
                      <small>{item.detail}</small>
                    </span>
                    {persona === item.id && <Icon name="check" size={14} />}
                  </button>
                ))}
              </div>
            </>
          )}

          {step === 1 && (
            <>
              <span className="onboarding-hero-icon">
                <Icon name="folder" size={23} />
              </span>
              <p className="eyebrow">Plain Markdown</p>
              <h1 id="onboarding-title" ref={headingRef} tabIndex={-1}>
                Pick your notes folder.
              </h1>
              <p className="onboarding-copy">
                Tooldeck reads and writes ordinary Markdown files. You can open
                them in any editor and change this folder later.
              </p>
              <button
                type="button"
                className="vault-picker"
                onClick={() => void chooseVault()}
              >
                <span className="drop-icon">
                  <Icon name="folder" size={18} />
                </span>
                <span>
                  <strong>{vault.split("/").at(-1)}</strong>
                  <small>{vault}</small>
                </span>
                <span className="secondary-button">Choose</span>
              </button>
              {vaultError && (
                <div className="inline-result error" role="alert">
                  <Icon name="circle-alert" size={16} />
                  <div>
                    <strong>Keep the current folder</strong>
                    <span>{vaultError}</span>
                  </div>
                </div>
              )}
            </>
          )}

          {step === 2 && (
            <>
              <span className="onboarding-hero-icon">
                <Icon name="settings" size={23} />
              </span>
              <p className="eyebrow">Optional access</p>
              <h1 id="onboarding-title" ref={headingRef} tabIndex={-1}>
                Add detail only when you want it.
              </h1>
              <p className="onboarding-copy">
                Tooldeck works without extra permissions. Window titles are
                optional and stay off until you enable them.
              </p>
              <ActivityPermissionPanel />
              <div className="onboarding-assurances">
                <span>
                  <Icon name="check" size={13} /> Idle detection needs no access
                </span>
                <span>
                  <Icon name="check" size={13} /> File prompts appear only when
                  you choose files
                </span>
                <span>
                  <Icon name="check" size={13} /> No account or network service
                </span>
              </div>
            </>
          )}
        </div>

        <footer className="onboarding-actions">
          <button type="button" className="text-button" onClick={finish}>
            Skip setup
          </button>
          <div>
            {step > 0 && (
              <button
                type="button"
                className="secondary-button"
                onClick={() => setStep((value) => value - 1)}
              >
                Back
              </button>
            )}
            <button
              type="button"
              className="primary-button"
              onClick={() => {
                if (step === 2) finish();
                else setStep((value) => value + 1);
              }}
            >
              {step === 2 ? "Open Tooldeck" : "Continue"}
              <Icon name="chevron-right" size={14} />
            </button>
          </div>
        </footer>
      </section>
    </div>
  );
}
