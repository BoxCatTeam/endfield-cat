import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import "@varlet/ui/es/style";
import { Snackbar } from "@varlet/ui";
import { createPinia } from "pinia";
import { initWindowState } from "./window/windowState";
import i18n from "./i18n";

Snackbar.setDefaultOptions({ position: "bottom" });

void initWindowState();
createApp(App).use(createPinia()).use(router).use(i18n).mount("#app");
