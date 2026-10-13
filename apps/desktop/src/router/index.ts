import { createRouter, createWebHashHistory } from "vue-router";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "translate",
      component: () => import("../views/translate/index.vue"),
      meta: { group: "工作台", title: "翻译" },
    },
    {
      path: "/history",
      name: "history",
      component: () => import("../views/history/index.vue"),
      meta: { group: "工作台", title: "历史" },
    },
    {
      path: "/wordbook",
      name: "wordbook",
      component: () => import("../views/wordbook/index.vue"),
      meta: { group: "工作台", title: "生词本" },
    },
    {
      path: "/service/rest",
      name: "rest-service",
      component: () => import("../views/rest-service/index.vue"),
      meta: { group: "服务", title: "REST API" },
    },
    {
      path: "/service/mcp",
      name: "mcp-service",
      component: () => import("../views/mcp-service/index.vue"),
      meta: { group: "服务", title: "MCP 服务" },
    },
    {
      path: "/lookup",
      name: "lookup",
      component: () => import("../views/lookup/index.vue"),
      meta: { layout: "bare" },
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("../views/settings/index.vue"),
      meta: { group: "配置", title: "设置" },
    },
  ],
});

export default router;
