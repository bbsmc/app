import dayjs from "dayjs";
// utc 必须在 timezone 之前加载
import utc from "dayjs/plugin/utc";
import timezone from "dayjs/plugin/timezone";
import relativeTime from "dayjs/plugin/relativeTime";
import "dayjs/locale/zh-cn";

dayjs.extend(utc);
dayjs.extend(timezone);
dayjs.extend(relativeTime);
// 设置 dayjs.tz() 不带参数时的默认时区为北京
dayjs.tz.setDefault("Asia/Shanghai");
dayjs.locale("zh-cn");

export default defineNuxtPlugin(() => {
  return {
    provide: {
      dayjs,
    },
  };
});
