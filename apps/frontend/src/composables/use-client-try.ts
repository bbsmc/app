type AsyncFunction<TArgs extends any[], TResult> = (...args: TArgs) => Promise<TResult>;
type ErrorFunction = (err: any) => void | Promise<void>;
type VoidFunction = () => void | Promise<void>;

type useClientTry = <TArgs extends any[], TResult>(
  fn: AsyncFunction<TArgs, TResult>,
  onFail?: ErrorFunction,
  onFinish?: VoidFunction,
) => (...args: TArgs) => Promise<TResult | undefined>;

const defaultOnError: ErrorFunction = (error) => {
  // 检测是否为封禁错误
  const errorName = error?.data?.error;

  if (errorName === "user_banned") {
    // 提供更友好的封禁提示
    const description = error?.data?.description || "您的账户已被封禁";
    addNotification({
      group: "main",
      title: "操作受限",
      text: `${description}。如有疑问，请前往账户设置查看详情或发起申诉。`,
      type: "error",
    });
    return;
  }

  // 检测是否为限流错误
  if (errorName === "ratelimit_error") {
    const description = error?.data?.description || "您的请求过于频繁";
    addNotification({
      group: "main",
      title: "请求过于频繁",
      text: `${description}，请稍后再试。`,
      type: "warn",
    });
    return;
  }

  addNotification({
    group: "main",
    title: "发生错误",
    text: error?.data?.description || error.message || error || "未知错误",
    type: "error",
  });
};

export const useClientTry: useClientTry =
  (fn, onFail = defaultOnError, onFinish) =>
  async (...args) => {
    startLoading();
    try {
      return await fn(...args);
    } catch (err) {
      if (onFail) {
        await onFail(err);
      } else {
        console.error("[CLIENT TRY ERROR]", err);
      }
    } finally {
      if (onFinish) await onFinish();
      stopLoading();
    }
  };
