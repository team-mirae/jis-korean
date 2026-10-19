import type { CSSProperties } from "react";
import type { Engine } from "./engine";

// The JIS kana-typing area (spec §2), with F6–F10 over 6–0. Offsets are in key units.
const ROWS: { offset: number; codes: string[] }[] = [
  { offset: 5, codes: ["F6", "F7", "F8", "F9", "F10"] },
  {
    offset: 0,
    codes: ["Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8", "Digit9", "Digit0", "Minus", "Equal", "IntlYen"],
  },
  { offset: 0.5, codes: [..."QWERTYUIOP"].map((l) => "Key" + l).concat("BracketLeft", "BracketRight") },
  { offset: 0.75, codes: [..."ASDFGHJKL"].map((l) => "Key" + l).concat("Semicolon", "Quote", "Backslash") },
  { offset: 1.25, codes: [..."ZXCVBNM"].map((l) => "Key" + l).concat("Comma", "Period", "Slash", "IntlRo") },
];

const FUNCTION_KEYS: Record<string, { label: string; planned?: boolean }> = {
  F6: { label: "한글" },
  F7: { label: "한자" },
  F8: { label: "두벌식" },
  F9: { label: "전각" },
  F10: { label: "반각" },
};

interface Props {
  engine: Engine;
  escaped: boolean;
  shift: boolean;
  pressed: Set<string>;
  onPress(code: string): void;
}

export function Keyboard({ engine, escaped, shift, pressed, onPress }: Props) {
  return (
    <div className="board">
      <div className="plate" role="group" aria-label="JIS Korean keyboard">
        {ROWS.map((row, i) => (
          <div key={i} className={"row" + (i === 0 ? " fn-row" : "")} style={{ "--offset": row.offset } as CSSProperties}>
            {row.codes.map((code) => {
              const fn = FUNCTION_KEYS[code];
              const base = fn ? code : engine.legend(code, false, escaped);
              const shifted = fn ? fn.label : engine.legend(code, true, escaped);
              const letter = code.startsWith("Key") ? code.slice(3) : "";
              const className = [
                "key",
                fn && "fn",
                fn?.planned && "planned",
                !letter && !fn && "sym",
                pressed.has(code) && "down",
                (code === "KeyF" || code === "KeyJ") && "home",
              ]
                .filter(Boolean)
                .join(" ");
              return (
                <button
                  key={code}
                  type="button"
                  className={className}
                  title={fn?.planned ? "Planned" : undefined}
                  aria-label={`${code}: ${base}${shifted && shifted !== base ? `, Shift: ${shifted}` : ""}`}
                  // Keep focus in the typing area so the composition isn't committed.
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => onPress(code)}
                >
                  {shifted !== base && <span className={"shifted" + (shift && !fn ? " live" : "")}>{shifted}</span>}
                  <span className="base">{base}</span>
                  {letter && <span className="latin">{letter}</span>}
                </button>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}
