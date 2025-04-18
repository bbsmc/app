<template>
  <div>
    <div class="universal-card">
      <div class="markdown-disclaimer">
        <h2>简介</h2>
        <span class="label__description">
          您可以在此处输入您的资源的详细描述。
          <span class="label__subdescription">
            描述必须清楚、诚实地描述目的和功能 项目。请参阅
            <nuxt-link to="/legal2/rules" class="text-link" target="_blank">内容规则</nuxt-link>
            满足全部要求。
          </span>
        </span>
      </div>
      <MarkdownEditor
        v-model="description"
        :on-image-upload="onUploadHandler"
        :disabled="(currentMember.permissions & EDIT_BODY) !== EDIT_BODY"
      />
      <div class="input-group markdown-disclaimer">
        <button
          type="button"
          class="iconified-button brand-button"
          :disabled="!hasChanges"
          @click="saveChanges()"
        >
          <SaveIcon />
          保存修改
        </button>
      </div>
    </div>
  </div>
</template>

<script>
import { MarkdownEditor } from "@modrinth/ui";
import Chips from "~/components/ui/Chips.vue";
import SaveIcon from "~/assets/images/utils/save.svg?component";
import { renderHighlightedString } from "~/helpers/highlight.js";
import { useImageUpload } from "~/composables/image-upload.ts";
const data = useNuxtApp();

export default defineNuxtComponent({
  components: {
    Chips,
    SaveIcon,
    MarkdownEditor,
  },
  props: {
    project: {
      type: Object,
      default() {
        return {};
      },
    },
    allMembers: {
      type: Array,
      default() {
        return [];
      },
    },
    currentMember: {
      type: Object,
      default() {
        return null;
      },
    },
    patchProject: {
      type: Function,
      default() {
        return () => {
          this.$notify({
            group: "main",
            title: "发生错误",
            text: "Patch project function not found",
            type: "error",
          });
        };
      },
    },
  },
  data() {
    return {
      description: this.project.body,
      bodyViewMode: "source",
    };
  },
  computed: {
    patchData() {
      const data = {};

      if (this.description !== this.project.body) {
        data.body = this.description;
      }

      return data;
    },
    hasChanges() {
      return Object.keys(this.patchData).length > 0;
    },
  },
  created() {
    this.EDIT_BODY = 1 << 3;
  },
  methods: {
    renderHighlightedString,
    saveChanges() {
      if (this.hasChanges) {
        this.patchProject(this.patchData);
      }
    },
    async onUploadHandler(file) {
      try {
        const response = await useImageUpload(file, {
          context: "project",
          projectID: this.project.id,
        });
        return response.url;
      } catch (e) {
        data.$notify({
          group: "main",
          title: "发生错误",
          text: e.data.description,
          type: "error",
        });
        return "";
      }
    },
  },
});
</script>

<style scoped>
.markdown-disclaimer {
  margin-block: 1rem;
}
</style>
