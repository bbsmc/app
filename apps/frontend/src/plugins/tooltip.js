import "floating-vue/dist/style.css";
import FloatingVue from "floating-vue";

// Upstream sync: 更新 floating-vue 5.x 配置
export default defineNuxtPlugin((nuxtApp) => {
  nuxtApp.vueApp.use(FloatingVue, {
    themes: {
      "ribbit-popout": {
        $extend: "dropdown",
        placement: "bottom-end",
        instantMove: true,
        distance: 8,
      },
      "dismissable-prompt": {
        $extend: "dropdown",
        placement: "bottom-start",
      },
    },
  });
});
