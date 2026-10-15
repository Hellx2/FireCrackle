import { useEffect } from "react";
import { CREDITS } from "../lib/credits";
import "./Credits.css";

interface Props {
  onClose: () => void;
}

/** The people FireCrackle's method comes from, and who built the tool. */
export function Credits({ onClose }: Props) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal credits" role="dialog" aria-modal="true" aria-label="Credits" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <h2>Credits</h2>
          <button onClick={onClose}>close</button>
        </div>
        <div className="modal-body credits-body">
          {CREDITS.map((section) => (
            <section key={section.title} className="credits-section">
              <h3>{section.title}</h3>
              <ul>
                {section.entries.map((e) => (
                  <li key={e.name}>
                    <b>{e.name}</b> {e.note}
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
