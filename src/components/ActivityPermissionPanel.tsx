import { useEffect, useState } from "react";

import {
  getActivityPermission,
  requestActivityPermission,
  type ActivityPermission,
} from "../lib/workflows";
import { Icon } from "./Icon";

export function ActivityPermissionPanel() {
  const [permission, setPermission] = useState<ActivityPermission | null>(null);
  const [requesting, setRequesting] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void getActivityPermission().then((value) => {
      if (!cancelled) setPermission(value);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  async function requestAccess() {
    setRequesting(true);
    try {
      setPermission(await requestActivityPermission());
    } finally {
      setRequesting(false);
    }
  }

  if (!permission) {
    return (
      <div className="permission-card permission-loading" aria-busy="true">
        <Icon name="loader" className="spin" size={16} />
        Checking local app-link detail…
      </div>
    );
  }

  return (
    <div className="permission-card">
      <span
        className={`permission-icon ${
          permission.windowTitlesAvailable ? "permission-on" : ""
        }`}
      >
        <Icon
          name={permission.windowTitlesAvailable ? "shield" : "circle-dashed"}
          size={17}
        />
      </span>
      <div>
        <strong>
          {permission.windowTitlesAvailable
            ? "Detailed app-linked tracking is on"
            : "App-name-only tracking"}
        </strong>
        <p>{permission.detail}</p>
      </div>
      {!permission.windowTitlesAvailable && (
        <button
          type="button"
          className="secondary-button"
          disabled={requesting}
          onClick={() => void requestAccess()}
        >
          {requesting ? "Opening…" : "Enable titles"}
        </button>
      )}
    </div>
  );
}
