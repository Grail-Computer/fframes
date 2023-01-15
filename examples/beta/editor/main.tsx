

import * as videoWasmBinding from "./editor-bridge/pkg/editor-bridge";
import { renderEditor } from "fframes-editor";
import "fframes-editor/tw.css";

renderEditor(
  import.meta.glob("../media/*", {
    as: "url",
    eager: true,
  }),
  videoWasmBinding
);
