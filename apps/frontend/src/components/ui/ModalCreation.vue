<template>
  <NewModal ref="modal" header="创建资源">
    <div class="flex flex-col gap-3">
      <div class="flex flex-col gap-2">
        <label for="name">
          <span class="text-lg font-semibold text-contrast">
            资源名称
            <span class="text-brand-red">*</span>
          </span>
        </label>
        <input
          id="name"
          v-model="name"
          type="text"
          maxlength="64"
          placeholder="资源名称..."
          autocomplete="off"
          @input="updatedName()"
        />
      </div>
      <div class="flex flex-col gap-2">
        <label for="slug" class="flex flex-col gap-1">
          <span class="text-lg font-semibold text-contrast">
            标识ID
            <span class="text-brand-red">*</span>
          </span>
          <span>请输入英文名,不可使用空格，特殊符号仅可使用 - 例如 ABC-FWAI-AF</span>
        </label>
        <div class="text-input-wrapper">
          <div class="text-input-wrapper__before">https://bbsmc.net/project/</div>
          <input
            id="slug"
            v-model="slug"
            type="text"
            maxlength="64"
            autocomplete="off"
            @input="manualSlug = true"
          />
        </div>
      </div>
      <div class="flex flex-col gap-2">
        <label for="visibility" class="flex flex-col gap-1">
          <span class="text-lg font-semibold text-contrast">
            可见度
            <span class="text-brand-red">*</span>
          </span>
          <span> 您的资源审核完成后的可见性。 </span>
        </label>
        <DropdownSelect
          id="visibility"
          v-model="visibility"
          :options="visibilities"
          :display-name="(x) => x.display"
          name="Visibility"
        />
      </div>
      <div class="flex flex-col gap-2">
        <label for="additional-information" class="flex flex-col gap-1">
          <span class="text-lg font-semibold text-contrast">
            简介
            <span class="text-brand-red">*</span>
          </span>
          <span> 一个简短的资源简介. </span>
        </label>
        <div class="textarea-wrapper">
          <textarea id="additional-information" v-model="description" maxlength="256" />
        </div>
      </div>

      <!-- 付费资源选项（仅高级创作者可见） -->
      <div v-if="isPremiumCreator" class="paid-section flex flex-col gap-2">
        <div class="flex items-center gap-2">
          <Checkbox v-model="isPaid" :label="'设为付费资源'" />
        </div>
        <template v-if="isPaid">
          <div class="flex flex-col gap-1">
            <label for="price" class="flex flex-col gap-1">
              <span class="text-md font-semibold text-contrast">
                价格
                <span class="text-brand-red">*</span>
              </span>
              <span class="text-sm text-secondary">设置价格（1-1000 元）</span>
            </label>
            <input
              id="price"
              v-model.number="price"
              type="number"
              min="1"
              max="1000"
              placeholder="请输入价格"
            />
          </div>
          <div class="flex flex-col gap-1">
            <label for="validity-days" class="flex flex-col gap-1">
              <span class="text-md font-semibold text-contrast">授权有效期</span>
              <span class="text-sm text-secondary">留空表示永久授权</span>
            </label>
            <input
              id="validity-days"
              v-model.number="validityDays"
              type="number"
              min="1"
              max="3650"
              placeholder="天数（可选，最多3650天）"
            />
          </div>
        </template>
      </div>

      <div class="flex gap-2">
        <ButtonStyled color="brand">
          <button @click="createProject">
            <PlusIcon aria-hidden="true" />
            创建资源
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button @click="cancel">
            <XIcon aria-hidden="true" />
            取消
          </button>
        </ButtonStyled>
      </div>
    </div>
  </NewModal>
</template>

<script setup>
import { NewModal, ButtonStyled, DropdownSelect, Checkbox } from "@modrinth/ui";
import { XIcon, PlusIcon } from "@modrinth/assets";

const router = useRouter();
const app = useNuxtApp();
// 直接获取已初始化的 auth 状态，避免 async 导致组件变成异步组件
const auth = useState("auth");

const props = defineProps({
  organizationId: {
    type: String,
    required: false,
    default: null,
  },
});

const modal = ref();

const name = ref("");
const slug = ref("");
const description = ref("");
const manualSlug = ref(false);

// 付费资源相关
const isPaid = ref(false);
const price = ref(null);
const validityDays = ref(null);

// 检查是否是高级创作者
const isPremiumCreator = computed(() => auth.value?.user?.is_premium_creator ?? false);

const visibilities = ref([
  {
    actual: "approved",
    display: "公开",
  },
  {
    actual: "unlisted",
    display: "不公开",
  },
  {
    actual: "private",
    display: "私有",
  },
]);
const visibility = ref({
  actual: "approved",
  display: "公开",
});

const cancel = () => {
  modal.value.hide();
};

// slug 验证正则：只允许字母、数字、连字符和部分特殊字符
const slugRegex = /^[a-zA-Z0-9!@$()`.+,_"-]+$/;

function validateSlug(value) {
  if (!value || value.trim() === "") {
    return "标识ID不能为空";
  }
  if (!slugRegex.test(value)) {
    return '标识ID只能包含英文字母、数字和部分特殊字符（!@$()`.+,_"-）';
  }
  return null;
}

async function createProject() {
  // 验证 slug
  const slugError = validateSlug(slug.value);
  if (slugError) {
    app.$notify({
      group: "main",
      title: "错误",
      text: slugError,
      type: "error",
    });
    return;
  }

  // 验证付费资源参数
  if (isPaid.value) {
    if (!price.value || !Number.isInteger(price.value) || price.value < 1 || price.value > 1000) {
      app.$notify({
        group: "main",
        title: "错误",
        text: "付费资源价格必须是 1-1000 之间的整数",
        type: "error",
      });
      return;
    }
    if (
      validityDays.value !== null &&
      (!Number.isInteger(validityDays.value) || validityDays.value < 1 || validityDays.value > 3650)
    ) {
      app.$notify({
        group: "main",
        title: "错误",
        text: "授权有效期必须是 1-3650 之间的整数",
        type: "error",
      });
      return;
    }
  }

  startLoading();

  const formData = new FormData();

  // 使用 v3 API 格式的项目数据
  const projectData = {
    name: name.value.trim(),
    slug: slug.value,
    summary: description.value.trim(),
    description: "",
    requested_status: visibility.value.actual,
    initial_versions: [],
    categories: [],
    license_id: "LicenseRef-Unknown",
    is_draft: true,
    // 付费资源字段（仅高级创作者可设置）
    is_paid: isPremiumCreator.value && isPaid.value,
    price: isPremiumCreator.value && isPaid.value ? price.value : null,
    validity_days:
      isPremiumCreator.value && isPaid.value && validityDays.value ? validityDays.value : null,
  };

  if (props.organizationId) {
    projectData.organization_id = props.organizationId;
  }

  formData.append("data", JSON.stringify(projectData));

  try {
    const result = await useBaseFetch("project", {
      method: "POST",
      body: formData,
      apiVersion: 3,
    });

    modal.value.hide();
    await router.push({
      name: "type-id",
      params: {
        type: "project",
        id: result.slug || result.id || slug.value,
      },
    });
  } catch (err) {
    app.$notify({
      group: "main",
      title: "发生错误",
      text: err.data?.description || err.message || "创建项目失败，请稍后重试",
      type: "error",
    });
  }
  stopLoading();
}

function show(event) {
  name.value = "";
  slug.value = "";
  description.value = "";
  manualSlug.value = false;
  isPaid.value = false;
  price.value = null;
  validityDays.value = null;
  modal.value.show(event);
}

defineExpose({
  show,
});

function updatedName() {
  if (!manualSlug.value) {
    slug.value = name.value
      .trim()
      .toLowerCase()
      .replaceAll(" ", "-")
      .replaceAll(/[^a-zA-Z0-9!@$()`.+,_"-]/g, "")
      .replaceAll(/--+/gm, "-");
  }
}
</script>

<style scoped>
.paid-section {
  padding: 1rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
}

.paid-section input[type="number"] {
  width: 100%;
  padding: 0.5rem 0.75rem;
  background: var(--color-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-sm);
}

.paid-section input[type="number"]:focus {
  outline: none;
  border-color: var(--color-brand);
}

.text-secondary {
  color: var(--color-text-secondary);
}
</style>
