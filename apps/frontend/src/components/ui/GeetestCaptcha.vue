<script>
export default {
  props: {
    modelValue: {
      type: String,
      required: true,
    },
  },
  data() {
    return {
      internalToken: this.modelValue,
      captchaId: 'fd0d19986b367cde0c815bd23567cb95',
      captchaObj: null,
    };
  },
  watch: {
    modelValue(newValue) {
      this.internalToken = newValue;
    },
  },
  mounted() {
    this.initGeetest();
  },
  methods: {
    async initGeetest() {
      const script = document.createElement('script');
      script.src = 'https://static.geetest.com/v4/gt4.js';
      document.body.appendChild(script);

      script.onload = () => {
        window.initGeetest4({
          captchaId: this.captchaId,
          product: 'float',
          language: 'zho',
          protocol: window.location.protocol === 'https:' ? 'https://' : 'http://',
        }, (captchaObj) => {
          this.captchaObj = captchaObj;

          // 注册事件监听
          captchaObj.onReady(() => {
            console.log('验证码已准备好');
          });

          captchaObj.onSuccess(() => {
            const result = captchaObj.getValidate();
            this.onTokenUpdate(result);
          });

          captchaObj.onError((e) => {
            console.error('Geetest error:', e);
          });

          captchaObj.onClose(() => {
            console.log('验证码关闭');
          });

          captchaObj.appendTo(document.getElementById('captcha'));
        });
      };
    },
    onTokenUpdate(token) {
      console.log('Token:', token);
      this.internalToken = token;
      this.$emit('update:modelValue', token);
    },
    // 暴露给父组件的方法
    reset() {
      if (this.captchaObj) {
        this.captchaObj.reset();
      }
    },
    // 获取验证结果
    getValidate() {
      if (this.captchaObj) {
        return this.captchaObj.getValidate();
      }
      return null;
    },
  },
  beforeUnmount() {
    if (this.captchaObj) {
      this.captchaObj.destroy();
    }
    const script = document.querySelector('script[src="https://static.geetest.com/v4/gt4.js"]');
    if (script) {
      document.body.removeChild(script);
    }
  },
};
</script>

<template>
  <div class="geetest-container">
    <div id="captcha"></div>
  </div>
</template>

<style scoped>
.geetest-container {
  display: flex;
  justify-content: center;
  align-items: center;
  width: 100%;
  min-height: 100px;
}

#captcha {
  margin: auto;
}
</style>
