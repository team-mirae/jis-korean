import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { loadEngine } from "./engine";
import "./styles.css";

const root = createRoot(document.getElementById("root")!);

loadEngine().then(
  (engine) =>
    root.render(
      <StrictMode>
        <App engine={engine} />
      </StrictMode>,
    ),
  (error) => {
    console.error(error);
    root.render(<p className="error">The engine failed to load.</p>);
  },
);
