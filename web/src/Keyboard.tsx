import type { CSSProperties } from "react";
import type { Engine } from "./engine";

type Row = { offset: number; codes: string[] };

// The JIS kana-typing area (spec §2), with F6–F10 over 6–0. Offsets are in key units.
const ROWS: Row[] = [
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

// Mode B (spec §3.6): the same area on US ANSI, laid out as on a MacBook: ` left of 1, \ (1.5 keys)
// right of ], and each row's offset after Tab, Caps Lock and Shift.
const ANSI_ROWS: Row[] = [
  { offset: 6, codes: ["F6", "F7", "F8", "F9", "F10"] },
  { offset: 0, codes: ["Backquote", ...ROWS[1].codes.slice(0, 12)] },
  { offset: 1.5, codes: [..."QWERTYUIOP"].map((l) => "Key" + l).concat("BracketLeft", "BracketRight", "Backslash") },
  { offset: 1.75, codes: [..."ASDFGHJKL"].map((l) => "Key" + l).concat("Semicolon", "Quote") },
  { offset: 2.25, codes: [..."ZXCVBNM"].map((l) => "Key" + l).concat("Comma", "Period", "Slash") },
];

interface Props {
  engine: Engine;
  /** Mode B: the US ANSI board. */
  ansi: boolean;
  escaped: boolean;
  shift: boolean;
  pressed: Set<string>;
  /** Float over the page (fixed to the bottom of the window) while the text is long. */
  floating: boolean;
  onPress(code: string): void;
}

export function Keyboard({ engine, ansi, escaped, shift, pressed, floating, onPress }: Props) {
  return (
    <div className={"board" + (ansi ? " ansi" : "") + (floating ? " floating" : "")}>
      <div className="plate" role="group" aria-label={ansi ? "JIS Korean for US ANSI keyboard" : "JIS Korean keyboard"}>
        {(ansi ? ANSI_ROWS : ROWS).map((row, i) => (
          <div key={i} className={"row" + (i === 0 ? " fn-row" : "")} style={{ "--offset": row.offset } as CSSProperties}>
            {row.codes.map((code) => {
              const fn = FUNCTION_KEYS[code];
              const base = fn ? code : engine.legend(code, false, escaped);
              // Shift+Z is F10 (half-width letters) outside the F8 escape.
              const shifted = fn ? fn.label : code === "KeyZ" && !escaped ? "반각" : engine.legend(code, true, escaped);
              const letter = code.startsWith("Key") ? code.slice(3) : "";
              const className = [
                "key",
                fn && "fn",
                fn?.planned && "planned",
                !letter && !fn && "sym",
                pressed.has(code) && "down",
                (code === "KeyF" || code === "KeyJ") && "home",
                ansi && code === "Backslash" && "wide",
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
