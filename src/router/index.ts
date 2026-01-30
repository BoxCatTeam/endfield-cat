import { createRouter, createWebHashHistory } from "vue-router";

// import MainPage from "../pages/MainPage.vue";
// import HomePage from "../pages/HomePage.vue";
// import LauncherPage from "../pages/LauncherPage.vue";
// import GachaPage from "../pages/GachaPage.vue";
// import SettingsPage from "../pages/SettingsPage.vue";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      component: () => import("../pages/MainPage.vue"),
      children: [
        { path: "", name: "home", component: () => import("../pages/HomePage.vue"), meta: { titleKey: "nav.home" } },
        { path: "launcher", name: "launcher", component: () => import("../pages/LauncherPage.vue"), meta: { titleKey: "nav.launcher" } },
        { path: "gacha", name: "gacha", component: () => import("../pages/GachaPage.vue"), meta: { titleKey: "nav.gacha" } },
        { path: "settings", name: "settings", component: () => import("../pages/SettingsPage.vue"), meta: { titleKey: "nav.settings" } },
      ],
    },
    {
      path: "/guide",
      name: "guide",
      component: () => import("../pages/GuidePage.vue"),
      meta: { titleKey: "guide.title" }
    },
  ],
});

export default router;
