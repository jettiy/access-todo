import "./app.css";
import App from "./App.svelte";
import DevNoteWidget from "./DevNoteWidget.svelte";
import Settings from "./Settings.svelte";
import { getWindowAgent } from "./api";

const target = document.getElementById("app");
if (!target) throw new Error("#app element missing");

const agent = getWindowAgent();

// devnotes 창은 개발노트 위젯, settings 창은 설정창, 나머지는 포스트잇
if (agent === "devnotes") {
  new DevNoteWidget({ target });
} else if (agent === "settings") {
  new Settings({ target });
} else {
  new App({ target });
}
