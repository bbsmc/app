<script>
export default {
  props: {
    modelValue: {
      type: String,
      required: true,
    },
    // TAC验证码配置选项
    type: {
      type: String,
      default: "SLIDER", // 默认使用滑块验证
    },
    requestUrl: {
      type: String,
      default: "https://captcha.bbsmc.net/gen",
    },
    validUrl: {
      type: String,
      default: "https://captcha.bbsmc.net/validation",
    },
    sdkUrl: {
      type: String,
      default: "https://captcha.bbsmc.net/sdk/tac",
    },
    logoUrl: {
      type: String,
      default:
        "https://cdn.bbsmc.net/bbsmc/data/ZcUCcMEr/317f155094c061b70526b21f83619037a4a962e7.png",
    },
    // 是否显示遮罩层
    showOverlay: {
      type: Boolean,
      default: false,
    },
  },
  data() {
    return {
      internalToken: this.modelValue,
      tacInstance: null,
      isLoaded: false,
      isVerifying: false, // 添加验证中状态
    };
  },
  mounted() {
    // 移除自动加载，改为按需加载
    this.loadTACScript();
  },
  beforeUnmount() {
    if (this.tacInstance) {
      this.tacInstance.destroyWindow();
    }
  },
  watch: {
    modelValue(newValue) {
      this.internalToken = newValue;
    },
    type() {
      // 当验证码类型改变时，如果正在验证则重新初始化
      if (this.isVerifying) {
        this.initCaptcha();
      }
    },
  },
  methods: {
    // 动态加载TAC验证码脚本
    async loadTACScript() {
      try {
        // 加载TAC load.js脚本
        await this.loadScript(`${this.sdkUrl}/load.js`);
        this.isLoaded = true;
      } catch (error) {
        console.error("TAC验证码脚本加载失败:", error);
      }
    },

    // 加载外部脚本的Promise包装
    loadScript(src) {
      return new Promise((resolve, reject) => {
        // 检查是否已经加载过
        if (document.querySelector(`script[src="${src}"]`)) {
          resolve();
          return;
        }

        const script = document.createElement("script");
        script.src = src;
        script.onload = resolve;
        script.onerror = reject;
        document.head.appendChild(script);
      });
    },

    // 初始化TAC验证码
    async initCaptcha() {
      if (!this.isLoaded || !window.initTAC) {
        return;
      }

      // 清理之前的实例
      if (this.tacInstance) {
        this.tacInstance.destroyWindow();
      }

      this.isVerifying = true;

      // 验证码配置
      const captchaConfig = {
        // 请求验证码接口
        requestCaptchaDataUrl: `${this.requestUrl}?type=${this.type}`,
        // 验证验证码接口
        validCaptchaUrl: this.validUrl,
        // 绑定的div
        bindEl: "#captcha-box",
        // 设置为true时传递的时间参数将转换成时间戳
        timeToTimestamp: false,
        // 验证成功回调函数
        validSuccess: (res, c, t) => {
          // 销毁验证码
          t.destroyWindow();
          this.isVerifying = false;
          // 更新token并回调给父组件
          this.onTokenUpdate(res.data.token);
        },
        // 验证失败回调函数
        validFail: (res, c, t) => {
          t.reloadCaptcha();
        },
        // 刷新按钮回调事件
        btnRefreshFun: (el, tac) => {
          tac.reloadCaptcha();
        },
        // 关闭按钮回调事件
        btnCloseFun: (el, tac) => {
          tac.destroyWindow();
          this.isVerifying = false;
        },
      };

      // 样式配置
      const style = {
        logoUrl: this.logoUrl,
      };

      try {
        // 初始化TAC验证码
        const tac = await window.initTAC(this.sdkUrl, captchaConfig, style);

        // 设置请求钩子
        tac.config.insertRequestChain(0, {
          // 请求前hook
          preRequest(type, requestParam) {
            return true;
          },
          // 请求后hook
          postRequest(type, requestParam, res) {
            return true;
          },
        });

        this.tacInstance = tac;

        // 调用初始化方法初始化验证码
        tac.init();
      } catch (error) {
        this.isVerifying = false;
      }
    },

    // Token更新回调 - 保留原有功能
    onTokenUpdate(token) {
      this.internalToken = token;
      this.$emit("update:modelValue", token);
    },

    // 手动触发验证码显示
    showCaptcha() {
      if (!this.isLoaded) {
        return;
      }
      this.initCaptcha();
    },

    // 重新加载验证码
    reloadCaptcha() {
      if (this.tacInstance) {
        this.tacInstance.reloadCaptcha();
      }
    },

    // 销毁验证码
    destroyCaptcha() {
      if (this.tacInstance) {
        this.tacInstance.destroyWindow();
        this.isVerifying = false;
      }
    },

    // 重置验证码状态
    resetCaptcha() {
      this.destroyCaptcha();
      this.tacInstance = null;
      this.isVerifying = false;
      this.onTokenUpdate("");
    },
  },
};
</script>

<template>
  <div class="tac-captcha-container">
    <!-- 可选的遮罩层 -->
    <div v-if="isVerifying && showOverlay" class="captcha-overlay" @click="destroyCaptcha"></div>

    <!-- 主要验证按钮 -->
    <div class="main-captcha-button">
      <!-- 未验证状态 - 点击开始验证 -->
      <button
        v-if="!internalToken && !isVerifying"
        @click="showCaptcha"
        class="captcha-btn captcha-btn-start"
        type="button"
      >
        <span class="btn-text">点击按钮开始验证</span>
      </button>

      <!-- 验证中状态 -->
      <button
        v-else-if="!internalToken && isVerifying"
        class="captcha-btn captcha-btn-verifying"
        type="button"
        disabled
      >
        <span class="btn-text">验证中...</span>
      </button>

      <!-- 验证成功状态 -->
      <button v-else class="captcha-btn captcha-btn-success" type="button" disabled>
        <span class="btn-text">验证通过</span>
        <span class="btn-icon">✓</span>
      </button>
    </div>

    <!-- TAC验证码容器 - 用于验证码渲染 -->
    <div id="captcha-box" class="captcha-box-container" v-show="isVerifying"></div>

    <!-- 验证码操作按钮（开发调试用） -->
    <div v-if="$attrs.showDebugControls" class="captcha-controls">
      <button
        @click="reloadCaptcha"
        class="btn btn-secondary"
        type="button"
        :disabled="!tacInstance"
      >
        刷新验证码
      </button>

      <button @click="destroyCaptcha" class="btn btn-danger" type="button" :disabled="!tacInstance">
        关闭验证码
      </button>

      <button @click="resetCaptcha" class="btn btn-warning" type="button">重置状态</button>
    </div>
  </div>
</template>

<style scoped>
.tac-captcha-container {
  width: 100%;
  position: relative;
}

/* 主要验证按钮样式 */
.main-captcha-button {
  margin: 15px 0;
  display: flex;
  justify-content: center;
}

.captcha-btn {
  width: 60%;
  height: 50px;
  border: 2px solid;
  border-radius: 6px;
  background: white;
  cursor: pointer;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 15px;
  transition: all 0.3s ease;
  position: relative;
  border-left: 4px solid transparent;
}

/* 点击开始验证按钮 - 灰色状态 */
.captcha-btn-start {
  border-color: #d1d5db;
  color: #6b7280;
  background-color: #f9fafb;
}

.captcha-btn-start:hover {
  background-color: #f3f4f6;
  border-color: #9ca3af;
  border-left-color: #3b82f6;
  color: #374151;
}

.captcha-btn-start:active {
  background-color: #e5e7eb;
}

/* 验证中状态按钮 - 蓝色 */
.captcha-btn-verifying {
  border-color: #3b82f6;
  color: #3b82f6;
  cursor: not-allowed;
  border-left-color: #3b82f6;
  background-color: #eff6ff;
}

/* 验证成功按钮 - 绿色 */
.captcha-btn-success {
  border-color: #10b981;
  color: #10b981;
  background-color: #ecfdf5;
  cursor: not-allowed;
  border-left-color: #10b981;
}

.btn-text {
  flex: 1;
  text-align: center;
  font-weight: 500;
}

.btn-icon {
  font-size: 18px;
  margin-left: 10px;
}

.captcha-box {
  margin: 10px 0;
  min-height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 验证码容器 - 弹窗样式 */
.captcha-box-container {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 10001;
  background-color: transparent;
  border-radius: 8px;
  max-width: 90vw;
  max-height: 90vh;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 验证码内容样式 */
.captcha-box-container > * {
  margin: 0 auto;
  display: block;
}

/* 可选遮罩层 */
.captcha-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.5);
  z-index: 10000;
  cursor: pointer;
}

/* 调试控制按钮 */
.captcha-controls {
  display: flex;
  gap: 10px;
  margin-top: 10px;
  padding: 10px;
  background-color: #f5f5f5;
  border-radius: 4px;
  border-left: 3px solid #ffc107;
}

.captcha-controls::before {
  content: "🛠️ 调试控制: ";
  font-size: 12px;
  color: #666;
  align-self: center;
  margin-right: 5px;
}

.btn {
  padding: 6px 12px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: background-color 0.3s;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-secondary {
  background-color: #6c757d;
  color: white;
}

.btn-secondary:hover:not(:disabled) {
  background-color: #545b62;
}

.btn-danger {
  background-color: #dc3545;
  color: white;
}

.btn-danger:hover:not(:disabled) {
  background-color: #c82333;
}

.btn-warning {
  background-color: #ffc107;
  color: #212529;
}

.btn-warning:hover:not(:disabled) {
  background-color: #e0a800;
}

/* TAC验证码弹窗样式调整 */
:deep(.tac-modal) {
  z-index: 9999 !important;
}

/* 响应式设计 */
@media (max-width: 480px) {
  .captcha-btn {
    height: 45px;
    font-size: 13px;
    padding: 0 12px;
  }

  .btn-icon {
    font-size: 16px;
  }

  .captcha-controls {
    flex-direction: column;
  }
}
</style>
