import "./styles/fonts.css";
import "./styles/design.css";
import "./styles/app.css";
import { wireControls } from "./ui/controls.ts";
import { wireDesktop } from "./ui/desktop.ts";
import { buildPoints } from "./ui/points.ts";
import { render } from "./ui/render.ts";
import { setRenderer } from "./ui/state.ts";
import { loadTheme } from "./ui/theme.ts";

loadTheme();
buildPoints(render);
wireControls();
wireDesktop();
setRenderer(render);
render();
