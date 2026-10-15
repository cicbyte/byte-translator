import { createApp } from "vue";
import Antd, { message } from "ant-design-vue";
import App from "./App.vue";
import router from "./router";
import "ant-design-vue/dist/reset.css";
import "./styles/global.css";

const app = createApp(App);

// 全局错误兜底：组件错误与未处理 Promise 拒绝不静默
app.config.errorHandler = (err, _instance, info) => {
  console.error(`[vue:${info}]`, err);
  message.error(`界面异常：${(err as Error)?.message ?? String(err)}`);
};
window.addEventListener("unhandledrejection", (ev) => {
  console.error("[unhandledrejection]", ev.reason);
  message.error(`操作失败：${(ev.reason as Error)?.message ?? String(ev.reason)}`);
});

app.use(router).use(Antd).mount("#app");
