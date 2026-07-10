import "./app.css";
import App from "./App.svelte";
import DevNoteWidget from "./DevNoteWidget.svelte";
import { getWindowAgent } from "./api";

const target = document.getElementById("app");
if (!target) throw new Error("#app element missing");

const agent = getWindowAgent();

// devnotes 창이면 개발노트 위젯 렌더링, 아니면 기존 포스트잇
if (agent === "devnotes") {
  new DevNoteWidget({ target });
} else {
  new App({ target });
}
