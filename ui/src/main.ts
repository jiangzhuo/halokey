import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { applyTheme, getTheme } from "./lib/theme";

applyTheme(getTheme());

mount(App, { target: document.getElementById("app")! });
