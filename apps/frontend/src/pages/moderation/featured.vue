<template>
  <div>
    <NewModal ref="editModal" :closable="!saving" :close-on-esc="!saving">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">
          {{ editing ? "编辑推荐" : `新增推荐 · ${sectionName}` }}
        </div>
      </template>
      <div class="modal-content featured-form">
        <div class="form-group">
          <label for="featured-project">关联项目（可选）</label>
          <div class="inline-input">
            <input
              id="featured-project"
              v-model="form.project"
              type="text"
              placeholder="项目 slug 或 ID，如 snk"
              autocomplete="off"
              maxlength="64"
            />
            <ButtonStyled>
              <button
                type="button"
                :disabled="!form.project.trim() || importing"
                @click="importProject"
              >
                {{ importing ? "导入中..." : "从项目导入" }}
              </button>
            </ButtonStyled>
          </div>
          <small class="hint"
            >导入会用项目标题、简介、精选图和链接填充下方字段，之后仍可手动修改。</small
          >
        </div>

        <div class="form-group">
          <label for="featured-title">标题 <span class="required">*</span></label>
          <input id="featured-title" v-model="form.title" type="text" maxlength="128" />
        </div>

        <div class="form-group">
          <label for="featured-description">描述</label>
          <textarea
            id="featured-description"
            v-model="form.description"
            rows="2"
            maxlength="512"
            placeholder="一句话介绍，显示在标题下方"
          />
        </div>

        <div v-if="currentSection === 'home'" class="form-group">
          <label for="featured-badge">角标</label>
          <input
            id="featured-badge"
            v-model="form.badge"
            type="text"
            maxlength="32"
            placeholder="如：精选整合包"
          />
          <small class="hint">仅首页轮播显示，留空则不显示角标。</small>
        </div>

        <div class="form-group">
          <label for="featured-link">跳转链接 <span class="required">*</span></label>
          <input
            id="featured-link"
            v-model="form.link"
            type="text"
            maxlength="2048"
            placeholder="/modpack/snk 或 https://..."
          />
          <small class="hint"
            >站内链接以 / 开头；站外链接需以 https:// 开头，会在新窗口打开。</small
          >
        </div>

        <div class="form-group">
          <label for="featured-image">图片 <span class="required">*</span></label>
          <div class="inline-input">
            <input
              id="featured-image"
              v-model="form.image_url"
              type="text"
              maxlength="2048"
              placeholder="https://cdn.bbsmc.net/..."
            />
            <FileInput
              class="iconified-button raised-button"
              :prompt="uploading ? '上传中...' : '上传图片'"
              accept="image/png,image/jpeg,image/gif,image/webp"
              :max-size="4194304"
              should-always-reset
              :disabled="uploading"
              aria-label="上传图片"
              @change="uploadImage"
            >
              <UploadIcon aria-hidden="true" />
            </FileInput>
          </div>
          <small class="hint"
            >建议 1600×800（2:1）以上、4MB 以内；也可直接填项目图库里的图片地址。</small
          >
          <div v-if="form.image_url.trim()" class="image-preview">
            <img :src="form.image_url.trim()" alt="图片预览" />
          </div>
        </div>

        <div class="form-group">
          <Checkbox
            v-model="form.enabled"
            label="启用"
            description="关闭后前台不显示，配置保留。"
          />
        </div>
      </div>
      <div class="modal-actions">
        <ButtonStyled color="brand">
          <button :disabled="saving" @click="saveItem">
            {{ saving ? "保存中..." : "保存" }}
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button :disabled="saving" @click="editModal?.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <ConfirmModal
      ref="deleteModal"
      title="删除这条推荐？"
      :description="`将从「${sectionName}」移除「${active?.title ?? ''}」，此操作不可撤销。`"
      proceed-label="确认删除"
      @proceed="doDelete"
    />

    <section class="universal-card">
      <div class="header-section">
        <div>
          <h2>推荐位</h2>
          <p class="description">
            管理首页 hero 轮播与各资源页顶部轮播。保存后前台立即生效（访客已打开的页面最多 5
            分钟后刷新）。
          </p>
        </div>
        <ButtonStyled color="brand">
          <button @click="openCreate"><PlusIcon aria-hidden="true" /> 新增推荐</button>
        </ButtonStyled>
      </div>

      <div class="tabs-wrapper">
        <NavTabs :links="tabLinks" query="section" />
      </div>

      <div v-if="itemsPending && items.length === 0" class="state-text">加载中...</div>
      <div v-else-if="itemsError" class="state-text">
        加载失败：{{ itemsError.data?.description || itemsError.message }}
        <ButtonStyled><button @click="refreshItems()">重试</button></ButtonStyled>
      </div>
      <div v-else-if="items.length === 0" class="state-text">
        「{{ sectionName }}」还没有推荐内容，点击右上角「新增推荐」添加。
      </div>
      <ul v-else class="featured-list">
        <li
          v-for="(item, index) in items"
          :key="item.id"
          class="featured-item"
          :class="{ disabled: !item.enabled }"
        >
          <div class="featured-order">{{ index + 1 }}</div>
          <img :src="item.image_url" :alt="item.title" class="featured-thumb" loading="lazy" />
          <div class="featured-info">
            <div class="featured-title-row">
              <span v-if="item.badge" class="featured-badge">{{ item.badge }}</span>
              <strong class="featured-title">{{ item.title }}</strong>
              <span v-if="!item.enabled" class="featured-status">已停用</span>
            </div>
            <p class="featured-desc">{{ item.description || "（无描述）" }}</p>
            <a
              :href="item.link"
              target="_blank"
              rel="noopener"
              class="featured-link"
              :title="item.link"
            >
              <LinkIcon aria-hidden="true" /> {{ item.link }}
            </a>
          </div>
          <div class="featured-actions">
            <div class="featured-move">
              <ButtonStyled circular type="transparent">
                <button
                  :disabled="index === 0 || reordering"
                  title="上移"
                  aria-label="上移"
                  @click="move(index, -1)"
                >
                  <DropdownIcon class="rotate-180" aria-hidden="true" />
                </button>
              </ButtonStyled>
              <ButtonStyled circular type="transparent">
                <button
                  :disabled="index === items.length - 1 || reordering"
                  title="下移"
                  aria-label="下移"
                  @click="move(index, 1)"
                >
                  <DropdownIcon aria-hidden="true" />
                </button>
              </ButtonStyled>
            </div>
            <ButtonStyled>
              <button :disabled="toggling === item.id" @click="toggleEnabled(item)">
                <EyeOffIcon v-if="item.enabled" aria-hidden="true" />
                <EyeIcon v-else aria-hidden="true" />
                {{ item.enabled ? "停用" : "启用" }}
              </button>
            </ButtonStyled>
            <ButtonStyled>
              <button @click="openEdit(item)"><EditIcon aria-hidden="true" /> 编辑</button>
            </ButtonStyled>
            <ButtonStyled color="red">
              <button @click="confirmDelete(item)"><TrashIcon aria-hidden="true" /> 删除</button>
            </ButtonStyled>
          </div>
        </li>
      </ul>
    </section>
  </div>
</template>

<script setup>
import { ref, computed } from "vue";
import { NewModal, ConfirmModal, ButtonStyled, Checkbox, FileInput } from "@modrinth/ui";
import {
  PlusIcon,
  EditIcon,
  TrashIcon,
  LinkIcon,
  UploadIcon,
  DropdownIcon,
  EyeIcon,
  EyeOffIcon,
} from "@modrinth/assets";
import NavTabs from "~/components/ui/NavTabs.vue";

const auth = await useAuth();

if (auth.value?.user?.role !== "admin") {
  throw createError({
    statusCode: 403,
    statusMessage: "权限不足",
    message: "只有超级管理员可以访问此页面",
  });
}

useHead({
  title: "推荐位 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const route = useNativeRoute();

const { data: sections } = await useAsyncData(
  "featured-admin-sections",
  () => useBaseFetch("featured/sections", { internal: true }),
  { default: () => [] },
);

const currentSection = computed(() => {
  const key = route.query.section;
  return typeof key === "string" && sections.value.some((s) => s.key === key) ? key : "home";
});

const sectionName = computed(
  () => sections.value.find((s) => s.key === currentSection.value)?.name ?? currentSection.value,
);

const tabLinks = computed(() =>
  sections.value.map((s) => ({ label: s.name, href: s.key === "home" ? "" : s.key })),
);

const {
  data: items,
  refresh: refreshItems,
  pending: itemsPending,
  error: itemsError,
} = await useAsyncData(
  "featured-admin-items",
  () => useBaseFetch(`featured/items?section=${currentSection.value}`, { internal: true }),
  { default: () => [], watch: [currentSection] },
);

const editModal = ref(null);
const deleteModal = ref(null);
const editing = ref(null);
const active = ref(null);
const saving = ref(false);
const uploading = ref(false);
const importing = ref(false);
const reordering = ref(false);
const toggling = ref(null);

const blankForm = () => ({
  project: "",
  title: "",
  description: "",
  badge: "",
  link: "",
  image_url: "",
  enabled: true,
});
const form = ref(blankForm());

const notifySuccess = (text) =>
  addNotification({ group: "main", title: "成功", text, type: "success" });
const notifyError = (title, error, fallback) => {
  console.error(title, error);
  addNotification({
    group: "main",
    title,
    text: error?.data?.description || fallback || "操作失败，请重试",
    type: "error",
  });
};

const afterChange = async (message) => {
  clearFeaturedCache();
  await refreshItems();
  notifySuccess(message);
};

function openCreate() {
  editing.value = null;
  form.value = blankForm();
  editModal.value?.show();
}

function openEdit(item) {
  editing.value = item;
  form.value = {
    project: item.project_id ?? "",
    title: item.title,
    description: item.description,
    badge: item.badge,
    link: item.link,
    image_url: item.image_url,
    enabled: item.enabled,
  };
  editModal.value?.show();
}

async function importProject() {
  const key = form.value.project.trim();
  if (!key) return;
  importing.value = true;
  try {
    const project = await useBaseFetch(`project/${encodeURIComponent(key)}`);
    const gallery = project.gallery ?? [];
    const cover = gallery.find((g) => g.featured) ?? gallery[0];
    form.value.project = project.id;
    form.value.title = project.title ?? form.value.title;
    form.value.description = project.description ?? form.value.description;
    form.value.link = `/${project.project_type}/${project.slug}`;
    if (cover?.url) {
      form.value.image_url = cover.url;
    }
    notifySuccess(
      cover?.url ? "已导入项目信息" : "已导入项目信息（该项目没有图库图片，请手动填写图片）",
    );
  } catch (error) {
    notifyError("导入失败", error, "项目不存在或无法访问");
  } finally {
    importing.value = false;
  }
}

async function uploadImage(files) {
  const file = files?.[0];
  if (!file) return;
  const ext = (file.type.split("/")[1] || "").toLowerCase();
  if (!["png", "jpeg", "gif", "webp"].includes(ext)) {
    notifyError("上传失败", null, "仅支持 png / jpeg / gif / webp 图片");
    return;
  }
  uploading.value = true;
  try {
    const res = await useBaseFetch(`featured/image?ext=${ext}`, {
      method: "POST",
      body: file,
      internal: true,
    });
    form.value.image_url = res.url;
    notifySuccess("图片已上传");
  } catch (error) {
    notifyError("上传失败", error);
  } finally {
    uploading.value = false;
  }
}

async function saveItem() {
  const payload = {
    title: form.value.title.trim(),
    description: form.value.description.trim(),
    badge: form.value.badge.trim(),
    link: form.value.link.trim(),
    image_url: form.value.image_url.trim(),
    enabled: form.value.enabled,
    project_id: form.value.project.trim(),
  };
  if (!payload.title || !payload.link || !payload.image_url) {
    notifyError("无法保存", null, "标题、跳转链接和图片为必填项");
    return;
  }
  saving.value = true;
  try {
    if (editing.value) {
      await useBaseFetch(`featured/items/${editing.value.id}`, {
        method: "PATCH",
        body: payload,
        internal: true,
      });
    } else {
      await useBaseFetch("featured/items", {
        method: "POST",
        body: { ...payload, section: currentSection.value },
        internal: true,
      });
    }
    editModal.value?.hide();
    await afterChange(editing.value ? "已保存修改" : "已新增推荐");
  } catch (error) {
    notifyError("保存失败", error);
  } finally {
    saving.value = false;
  }
}

async function toggleEnabled(item) {
  toggling.value = item.id;
  try {
    await useBaseFetch(`featured/items/${item.id}`, {
      method: "PATCH",
      body: { enabled: !item.enabled },
      internal: true,
    });
    await afterChange(item.enabled ? "已停用，前台不再显示" : "已启用");
  } catch (error) {
    notifyError("操作失败", error);
  } finally {
    toggling.value = null;
  }
}

function confirmDelete(item) {
  active.value = item;
  deleteModal.value?.show();
}

async function doDelete() {
  if (!active.value) return;
  try {
    await useBaseFetch(`featured/items/${active.value.id}`, {
      method: "DELETE",
      internal: true,
    });
    await afterChange("已删除");
  } catch (error) {
    notifyError("删除失败", error);
  } finally {
    active.value = null;
  }
}

async function move(index, delta) {
  const target = index + delta;
  if (target < 0 || target >= items.value.length) return;
  const ids = items.value.map((x) => x.id);
  [ids[index], ids[target]] = [ids[target], ids[index]];
  reordering.value = true;
  try {
    items.value = await useBaseFetch("featured/items/reorder", {
      method: "POST",
      body: { section: currentSection.value, ids },
      internal: true,
    });
    clearFeaturedCache();
  } catch (error) {
    notifyError("排序失败", error, "排序失败，列表可能已变化，请刷新后重试");
    await refreshItems();
  } finally {
    reordering.value = false;
  }
}
</script>

<style scoped lang="scss">
.header-section {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--gap-md);
  margin-bottom: var(--gap-md);

  h2 {
    margin: 0;
  }

  .description {
    margin: var(--gap-xs) 0 0;
    color: var(--color-secondary);
  }
}

.tabs-wrapper {
  margin-bottom: var(--gap-lg);
}

.state-text {
  padding: var(--gap-xl) 0;
  text-align: center;
  color: var(--color-secondary);
}

.featured-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--gap-sm);
}

.featured-item {
  display: grid;
  grid-template-columns: auto 160px minmax(0, 1fr) auto;
  gap: var(--gap-md);
  align-items: center;
  padding: var(--gap-sm) var(--gap-md);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  background: var(--color-bg);

  &.disabled {
    opacity: 0.6;
  }
}

.featured-order {
  width: 1.5rem;
  text-align: center;
  font-weight: 700;
  color: var(--color-secondary);
}

.featured-thumb {
  width: 160px;
  height: 80px;
  object-fit: cover;
  border-radius: var(--radius-sm);
  background: var(--color-raised-bg);
}

.featured-info {
  min-width: 0;
}

.featured-title-row {
  display: flex;
  align-items: center;
  gap: var(--gap-xs);
  flex-wrap: wrap;
}

.featured-title {
  font-size: 1rem;
}

.featured-badge,
.featured-status {
  padding: 0.1rem 0.5rem;
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  font-weight: 600;
}

.featured-badge {
  background: var(--color-brand-highlight);
  color: var(--color-brand);
}

.featured-status {
  background: var(--color-orange-highlight, var(--color-raised-bg));
  color: var(--color-orange);
}

.featured-desc {
  margin: var(--gap-xs) 0;
  color: var(--color-secondary);
  font-size: 0.9rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.featured-link {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  max-width: 100%;
  font-size: 0.85rem;
  color: var(--color-link);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  svg {
    flex-shrink: 0;
    width: 1em;
    height: 1em;
  }
}

.featured-actions {
  display: flex;
  align-items: center;
  gap: var(--gap-xs);
  flex-wrap: wrap;
  justify-content: flex-end;
}

.featured-move {
  display: flex;
  flex-direction: column;
}

.modal-content {
  padding: 0 1rem;
  width: min(36rem, 90vw);

  .hint {
    display: block;
    margin-top: 0.35rem;
    color: var(--color-text-secondary);
    font-size: 0.85rem;
  }

  .required {
    color: var(--color-red);
  }
}

.form-group {
  margin: 1rem 0;

  label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 600;
  }

  input[type="text"],
  textarea {
    width: 100%;
    box-sizing: border-box;
  }

  textarea {
    padding: 0.5rem;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-sm);
    resize: vertical;
  }
}

.inline-input {
  display: flex;
  gap: var(--gap-xs);
  align-items: center;

  input {
    flex: 1;
    min-width: 0;
  }
}

.image-preview {
  margin-top: 0.5rem;
  border-radius: var(--radius-md);
  overflow: hidden;
  border: 1px solid var(--color-divider);
  aspect-ratio: 2 / 1;
  background: var(--color-raised-bg);

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
}

.modal-actions {
  display: flex;
  gap: 0.5rem;
  padding: 1rem;
  justify-content: flex-end;
}

@media screen and (max-width: 900px) {
  .featured-item {
    grid-template-columns: auto minmax(0, 1fr);
  }

  .featured-thumb {
    display: none;
  }

  .featured-actions {
    grid-column: 1 / -1;
    justify-content: flex-start;
  }

  .featured-move {
    flex-direction: row;
  }
}
</style>
