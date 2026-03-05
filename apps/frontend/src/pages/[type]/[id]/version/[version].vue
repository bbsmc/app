<template>
  <div v-if="version" class="version-page">
    <ConfirmModal
      v-if="currentMember"
      ref="modal_confirm"
      title="您确实要删除该版本吗？"
      description="这将永远删除该版本"
      :has-to-type="false"
      proceed-label="删除"
      @proceed="deleteVersion()"
    />
    <ConfirmModal2
      v-if="currentMember && version.disk_only && !version.is_modpack"
      ref="modal_confirm_create"
      title="这是否是整合包资源?"
      description="您使用了第三方网盘上传，但未选择是否是整合包资源，请二次确认是否是整合包资源"
      proceed-label="是整合包"
      cancel-label="不是整合包"
      @proceed="
        () => {
          version.is_modpack = true;
          createVersion();
        }
      "
      @reject="
        () => {
          version.is_modpack = false;
          createVersion();
        }
      "
    />
    <UploadModal
      ref="uploading_modal"
      title="文件上传中"
      :description="
        parseFloat(uploading).toFixed(0) === '100'
          ? '文件处理中,请稍等'
          : `当前进度 ${parseFloat(uploading).toFixed(2)}%`
      "
      :speed="
        parseFloat(uploading).toFixed(0) === '100'
          ? ''
          : `上传速度 ${parseFloat(uploadSpeed).toFixed(2)} MB/s`
      "
    />

    <!-- 重新提交审核对话框 -->
    <NewModal ref="resubmitModal">
      <template #title>
        <div class="truncate text-lg font-extrabold text-contrast">重新提交审核</div>
      </template>
      <div class="resubmit-content">
        <p class="text-secondary">请说明您重新提交的原因，以便审核者了解您的修改：</p>
        <textarea
          v-model="resubmitReason"
          class="resubmit-textarea"
          placeholder="例如：已修正翻译错误、已更新版本兼容性、已补充说明信息等..."
          rows="5"
          required
        ></textarea>
      </div>
      <div class="modal-actions">
        <ButtonStyled color="brand">
          <button :disabled="!resubmitReason || resubmittingLink" @click="confirmResubmit">
            <UndoIcon aria-hidden="true" />
            确认重新提交
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button @click="$refs.resubmitModal.hide()">取消</button>
        </ButtonStyled>
      </div>
    </NewModal>

    <NewModal ref="downloadModal">
      <template #title>
        <Avatar :src="project.icon_url" :alt="project.title" class="icon" size="32px" />
        <div class="truncate text-lg font-extrabold text-contrast">下载 {{ version.name }}</div>
      </template>

      <AutomaticAccordion div class="flex flex-col gap-2" style="width: 650px">
        <VersionSummary
          v-if="version"
          :version="version"
          @on-navigate="$refs.downloadModal.hide"
          @on-download="onDownload(version.id)"
        />

        <!-- 汉化包推荐 -->
        <TranslationPromo
          v-if="translationVersions.length > 0"
          :translation-version="translationVersions"
          @navigate="navigateToTranslation"
        />

        <!-- 汉化包未及时更新提示 -->
        <div
          v-else-if="project.translation_tracker && translationVersions.length === 0"
          class="translation-pending-notice border-orange-500/50 bg-orange-500/10 rounded-2xl border border-solid p-4"
        >
          <div class="flex items-start gap-3">
            <InfoIcon class="text-orange-400 mt-0.5 size-5 shrink-0" />
            <div class="flex flex-col gap-1">
              <span class="font-bold text-contrast">当前版本暂无汉化包</span>
              <span class="text-sm text-secondary">
                该版本的汉化包还未及时上传，可前往 QQ 群
                <span class="text-orange-400 font-mono font-bold">1073724937</span>
                反馈，我们将及时响应处理。
              </span>
            </div>
          </div>
        </div>
      </AutomaticAccordion>
    </NewModal>

    <div class="version-page__title universal-card">
      <Breadcrumbs
        :current-title="version.name"
        :link-stack="[
          {
            href: getPreviousLink(),
            label: getPreviousLabel(),
          },
        ]"
      />
      <div class="version-header">
        <template v-if="isEditing">
          <input
            v-model="version.name"
            type="text"
            placeholder="输入该版本的标题..."
            maxlength="256"
          />
        </template>
        <h2 :class="{ 'sr-only': isEditing }">
          {{ version.name }}
        </h2>
        <div v-if="version.featured" class="featured">
          <StarIcon aria-hidden="true" />
          精选
        </div>
        <div v-else-if="featuredVersions.find((x) => x.id === version.id)" class="featured">
          <StarIcon aria-hidden="true" />
          自动推荐
        </div>
      </div>
      <div v-if="fieldErrors && showKnownErrors" class="known-errors">
        <ul>
          <li v-if="version.version_number === ''">您必须输入一个版本号</li>
          <li
            v-if="
              version.disk_only &&
              (version.quark_disk === '' || version.quark_disk === undefined) &&
              (version.baidu_disk === '' || version.baidu_disk === undefined) &&
              (version.modrinth === '' || version.modrinth === undefined) &&
              (version.curseforge === '' || version.curseforge === undefined) &&
              (version.xunlei_disk === '' || version.xunlei_disk === undefined)
            "
          >
            您选择了提供第三方链接，必须提供至少一个地址
          </li>
          <li v-if="version.game_versions.length === 0 && version.type === 'minecraft'">
            您必须选择支持的 Minecraft 版本
          </li>
          <li
            v-if="
              newFiles.length === 0 &&
              version.files.length === 0 &&
              !replaceFile &&
              !version.disk_only
            "
          >
            您必须要有一个上传的文件
          </li>
          <li
            v-if="
              version.loaders.length === 0 &&
              project.project_type !== 'resourcepack' &&
              version.type !== 'language'
            "
          >
            您的版本必须选择资源的运行环境
          </li>
        </ul>
      </div>
      <div v-if="isCreating" class="input-group">
        <ButtonStyled color="brand">
          <button
            :disabled="shouldPreventActions"
            @click="
              version.disk_only && !version.is_modpack
                ? $refs.modal_confirm_create.show()
                : createVersion()
            "
          >
            <PlusIcon aria-hidden="true" />
            创建
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <nuxt-link
            v-if="auth.user"
            :to="`/${project.project_type}/${project.slug ? project.slug : project.id}/versions`"
          >
            <CrossIcon aria-hidden="true" />
            取消
          </nuxt-link>
        </ButtonStyled>
      </div>
      <div v-else-if="isEditing" class="input-group">
        <ButtonStyled color="brand">
          <button :disabled="shouldPreventActions" @click="saveEditedVersion">
            <SaveIcon aria-hidden="true" />
            保存
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button @click="version.featured = !version.featured">
            <StarIcon aria-hidden="true" />
            <template v-if="!version.featured"> 精选版本</template>
            <template v-else> 取消精选</template>
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <nuxt-link
            v-if="currentMember"
            class="action"
            :to="`/${project.project_type}/${
              project.slug ? project.slug : project.id
            }/version/${encodeURI(version.displayUrlEnding)}`"
          >
            <CrossIcon aria-hidden="true" />
            放弃修改
          </nuxt-link>
        </ButtonStyled>
      </div>
      <div v-else class="input-group">
        <ButtonStyled v-if="primaryFile" color="green">
          <!--          <a-->
          <!--            v-if="primaryFile.url.includes('cdn.bbsmc.net')"-->
          <!--            v-tooltip="primaryFile.filename + ' (' + $formatBytes(primaryFile.size) + ')'"-->
          <!--            :href="primaryFile.url"-->
          <!--            @click="emit('onDownload')"-->
          <!--          >-->
          <!--            <DownloadIcon aria-hidden="true" />-->
          <!--            下载-->
          <!--          </a>-->
          <button-styled color="green" @click="$refs.downloadModal.show()">
            <nuxt-link>
              <DownloadIcon aria-hidden="true" />
              下载
            </nuxt-link>
          </button-styled>
        </ButtonStyled>
        <ButtonStyled v-if="!auth.user">
          <nuxt-link to="/auth/sign-in">
            <ReportIcon aria-hidden="true" />
            举报反馈
          </nuxt-link>
        </ButtonStyled>
        <ButtonStyled v-else-if="!currentMember">
          <button @click="() => reportVersion(version.id)">
            <ReportIcon aria-hidden="true" />
            举报反馈
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <nuxt-link
            v-if="currentMember"
            class="action"
            :to="`/${project.project_type}/${
              project.slug ? project.slug : project.id
            }/version/${encodeURI(version.displayUrlEnding)}/edit`"
          >
            <EditIcon aria-hidden="true" />
            编辑
          </nuxt-link>
        </ButtonStyled>
        <ButtonStyled>
          <button v-if="currentMember" @click="$refs.modal_confirm.show()">
            <TrashIcon aria-hidden="true" />
            删除
          </button>
        </ButtonStyled>
      </div>
    </div>

    <div class="version-page__changelog universal-card">
      <h3>更新日志</h3>
      <template v-if="isEditing">
        <div class="changelog-editor-spacing">
          <MarkdownEditor v-model="version.changelog" :on-image-upload="onImageUpload" />
        </div>
      </template>
      <div
        v-else
        class="markdown-body"
        v-html="version.changelog ? renderHighlightedString(version.changelog) : '无'"
      />
    </div>

    <!-- 翻译该版本的资源列表 -->
    <div
      v-if="translationVersions.length > 0 || translationVersionsLoading"
      class="version-page__translations universal-card"
    >
      <h3>汉化包</h3>

      <!-- 加载指示器 -->
      <div v-if="translationVersionsLoading" class="loading-indicator">
        <span>正在加载翻译版本信息...</span>
      </div>

      <!-- 翻译版本列表（每个项目只显示最新版本） -->
      <div
        v-for="(translation, index) in translationVersions"
        v-show="!translationVersionsLoading"
        :key="index"
        class="translation-item button-transparent"
        @click="$router.push(translation.link)"
      >
        <Avatar
          :src="translation.project ? translation.project.icon_url : null"
          alt="translation-icon"
          size="sm"
        />
        <nuxt-link :to="translation.link" class="info">
          <span class="project-title">
            {{ translation.project ? translation.project.title : "加载中..." }}
          </span>
          <span class="translation-details">
            <span class="version-info"> v{{ translation.version_number }} </span>
            <span class="separator">·</span>
            <span class="language-info">
              {{
                translation.language_code === "zh_CN"
                  ? "简体中文"
                  : translation.language_code === "zh_TW"
                    ? "繁体中文"
                    : translation.language_code === "en_US"
                      ? "英语"
                      : translation.language_code === "ja_JP"
                        ? "日语"
                        : translation.language_code === "ko_KR"
                          ? "韩语"
                          : translation.language_code
              }}
            </span>
            <span class="separator">·</span>
            <span v-tooltip="formatDateTime(translation.date_published)" class="date-info">
              {{ fromNow(translation.date_published) }}
            </span>
          </span>
        </nuxt-link>
      </div>

      <div
        v-if="translationVersions.length === 0 && !translationVersionsLoading"
        class="no-translations"
      >
        <InfoIcon aria-hidden="true" />
        <span>暂无翻译版本</span>
      </div>
    </div>

    <div
      v-if="isEditing && version.type !== 'language'"
      class="version-page__disk_url universal-card"
    >
      <div class="adjacent-input">
        <label>
          <span class="label__title">第三方链接(网盘)下载</span>
          <span class="label__description">
            如果您有夸克迅雷等网盘的合作，需要参与网盘的拉新激励，可选择只提供网盘链接，则无需选择要上传的文件，提供网盘下载方式后，点击下载按钮会直接跳转到网盘进行下载便可得到网盘的拉新激励。
            如果您启用网盘提供下载，如果你希望提供本站下载+网盘下载，请你在版本列表页面点击
            版本上传(文件) 按钮来创建上传页面，不然文件会被识别为附加文件
            <br />
            <br />
            请至少提供一种下载方式
          </span>
        </label>
        <input
          id="advanced-rendering"
          v-model="version.disk_only"
          class="switch stylized-toggle"
          type="checkbox"
        />
      </div>

      <div v-if="version.disk_only === true">
        <h3>夸克网盘</h3>
        <input
          id="version-quark"
          v-model="version.quark_disk"
          placeholder="直接链接，不要设置网盘访问密码，否则无法正常跳转"
          type="text"
          autocomplete="off"
          style="width: 100%"
        />
        <h3>迅雷网盘</h3>
        <input
          id="version-xunlei"
          v-model="version.xunlei_disk"
          placeholder="直接链接，不要设置网盘访问密码，否则无法正常跳转"
          type="text"
          autocomplete="off"
          style="width: 100%"
        />
        <h3>百度网盘</h3>
        <input
          id="version-baidu"
          v-model="version.baidu_disk"
          placeholder="直接链接，不要设置网盘访问密码，否则无法正常跳转"
          type="text"
          autocomplete="off"
          style="width: 100%"
        />
        <h3>Modrinth版本页面(转载)</h3>
        <input
          id="version-modrinth"
          v-model="version.modrinth"
          placeholder="如果是转载资源，请将该地址填写成对应版本的子页面"
          type="text"
          autocomplete="off"
          style="width: 100%"
        />
        <h3>CurseForge版本页面(转载)</h3>
        <input
          id="version-curseforge"
          v-model="version.curseforge"
          placeholder="如果是转载资源，请将该地址填写成对应版本的子页面"
          type="text"
          autocomplete="off"
          style="width: 100%"
        />
        <div class="adjacent-input">
          <label style="margin-top: 15px">
            <span class="label__title">整合包</span>
            <span class="label__description">
              请选择上传的资源是否是整合包/导入包类型，这很重要
            </span>
          </label>
          <input
            id="advanced-rendering"
            v-model="version.is_modpack"
            type="checkbox"
            class="switch stylized-toggle"
          />
        </div>
      </div>
    </div>

    <div
      v-if="
        deps.length > 0 ||
        (isEditing && project.project_type !== 'modpack' && version.type !== 'language')
      "
      class="version-page__dependencies universal-card"
    >
      <h3>依赖项目</h3>
      <div
        v-for="(dependency, index) in deps.filter((x) => !x.file_name)"
        :key="index"
        class="dependency"
        :class="{ 'button-transparent': !isEditing }"
        @click="!isEditing ? $router.push(dependency.link) : {}"
      >
        <Avatar
          :src="dependency.project ? dependency.project.icon_url : null"
          alt="dependency-icon"
          size="sm"
        />
        <nuxt-link v-if="!isEditing" :to="dependency.link" class="info">
          <span class="project-title">
            {{ dependency.project ? dependency.project.title : "Unknown Project" }}
          </span>
          <span v-if="dependency.version" class="dep-type" :class="dependency.dependency_type">
            Version {{ dependency.version.version_number }} is
            {{ dependency.dependency_type }}
          </span>
          <span v-else class="dep-type" :class="dependency.dependency_type">
            {{ dependency.dependency_type }}
          </span>
        </nuxt-link>
        <div v-else class="info">
          <span class="project-title">
            {{ dependency.project ? dependency.project.title : "Unknown Project" }}
          </span>
          <span v-if="dependency.version" class="dep-type" :class="dependency.dependency_type">
            Version {{ dependency.version.version_number }} is
            {{ dependency.dependency_type }}
          </span>
          <span v-else class="dep-type" :class="dependency.dependency_type">
            {{ dependency.dependency_type }}
          </span>
        </div>
        <ButtonStyled v-if="isEditing && project.project_type !== 'modpack'">
          <button @click="version.dependencies.splice(index, 1)">
            <TrashIcon aria-hidden="true" />
            Remove
          </button>
        </ButtonStyled>
      </div>
      <div
        v-for="(dependency, index) in deps.filter((x) => x.file_name)"
        :key="index"
        class="dependency"
      >
        <Avatar :src="null" alt="dependency-icon" size="sm" />
        <div class="info">
          <span class="project-title">
            {{ dependency.file_name }}
          </span>
          <span class="dep-type" :class="dependency.dependency_type">Added via overrides</span>
        </div>
      </div>
      <div v-if="isEditing && project.project_type !== 'modpack'" class="add-dependency">
        <h4>添加在本网站发布过的资源作为依赖</h4>
        <div class="input-group">
          <Multiselect
            v-model="dependencyAddMode"
            class="input"
            :options="['project', 'version']"
            :custom-label="
              (value) => {
                switch (value) {
                  case 'project':
                    return '资源';
                  case 'version':
                    return '版本';
                  default:
                    return value.charAt(0).toUpperCase() + value.slice(1);
                }
              }
            "
            :searchable="false"
            :close-on-select="true"
            :show-labels="false"
            :allow-empty="false"
          />
          <input
            v-model="newDependencyId"
            type="text"
            :placeholder="`请输入 ${dependencyAddMode} ID${
              dependencyAddMode === 'project' ? '/slug' : ''
            }`"
            @keyup.enter="addDependency(dependencyAddMode, newDependencyId, newDependencyType)"
          />
          <Multiselect
            v-model="newDependencyType"
            class="input"
            :options="['required', 'optional', 'incompatible', 'embedded']"
            :custom-label="
              (value) => {
                switch (value) {
                  case 'required':
                    return '必需';
                  case 'optional':
                    return '可选';
                  case 'incompatible':
                    return '不兼容';
                  case 'embedded':
                    return '嵌入';
                  default:
                    return value.charAt(0).toUpperCase() + value.slice(1);
                }
              }
            "
            :searchable="false"
            :close-on-select="true"
            :show-labels="false"
            :allow-empty="true"
          />
        </div>
        <div class="input-group">
          <ButtonStyled color="brand">
            <button @click="addDependency(dependencyAddMode, newDependencyId, newDependencyType)">
              <PlusIcon aria-hidden="true" />
              添加依赖
            </button>
          </ButtonStyled>
        </div>
      </div>
    </div>

    <!-- 版本链接部分（仅用于语言类型） -->
    <!-- 版本链接卡片：只在有链接数据（且已加载）或编辑/创建模式时显示 -->
    <div
      v-if="
        (version.type === 'language' &&
          ((versionLinks.length > 0 && versionLinks.some((link) => link.originalVersion)) ||
            isEditing ||
            versionLinksLoading)) ||
        (isCreating && version.type === 'language')
      "
      class="version-page__version-links universal-card"
    >
      <h3>目标翻译资源版本</h3>
      <!-- 只显示已加载完成的版本链接，或在编辑模式下显示所有 -->
      <template v-for="(link, index) in versionLinks" :key="index">
        <div
          v-show="link.originalVersion || isEditing"
          class="version-link"
          :class="{ 'button-transparent': !isEditing }"
          @click="!isEditing && link.originalVersion ? $router.push(link.originalVersion.link) : {}"
        >
          <Avatar
            :src="link.originalVersion ? link.originalVersion.project.icon_url : null"
            alt="version-link-icon"
            size="sm"
          />
          <nuxt-link
            v-if="!isEditing && link.originalVersion"
            :to="link.originalVersion.link"
            class="info"
          >
            <span class="project-title">
              {{ link.originalVersion.project.title }}
            </span>
            <span class="link-details">
              版本 {{ link.originalVersion.version_number }} -
              {{ link.language_code === "zh_CN" ? "简体中文" : link.language_code }}
            </span>
          </nuxt-link>
          <!-- 审核状态标签（仅对项目成员显示） -->
          <span
            v-if="!isEditing && currentMember && link.approval_status"
            class="approval-status-tag"
            :class="{
              'status-approved':
                link.approval_status === 'approved' ||
                link.approval_status?.toLowerCase() === 'approved',
              'status-pending':
                link.approval_status === 'pending' ||
                link.approval_status?.toLowerCase() === 'pending',
              'status-rejected':
                link.approval_status === 'rejected' ||
                link.approval_status?.toLowerCase() === 'rejected',
            }"
            :title="`审核状态: ${link.approval_status}`"
          >
            {{ getApprovalStatusLabel(link.approval_status) }}
          </span>
          <div v-else-if="isEditing" class="info">
            <span class="project-title">
              {{ link.originalVersion ? link.originalVersion.project.title : "加载中..." }}
            </span>
            <span class="link-details">
              版本
              {{
                link.originalVersion ? link.originalVersion.version_number : link.joining_version_id
              }}
              - {{ link.language_code === "zh_CN" ? "简体中文" : link.language_code }}
            </span>
          </div>
          <!-- 审核状态标签（编辑模式下也显示） -->
          <span
            v-if="isEditing && currentMember && link.approval_status"
            class="approval-status-tag"
            :class="{
              'status-approved':
                link.approval_status === 'approved' ||
                link.approval_status?.toLowerCase() === 'approved',
              'status-pending':
                link.approval_status === 'pending' ||
                link.approval_status?.toLowerCase() === 'pending',
              'status-rejected':
                link.approval_status === 'rejected' ||
                link.approval_status?.toLowerCase() === 'rejected',
            }"
            :title="`审核状态: ${link.approval_status}`"
          >
            {{ getApprovalStatusLabel(link.approval_status) }}
          </span>
          <ButtonStyled v-if="isEditing">
            <button @click="removeVersionLink(index)">
              <TrashIcon aria-hidden="true" />
              移除
            </button>
          </ButtonStyled>
          <!-- 重新提交按钮（被拒绝的链接且是项目成员时显示） -->
          <button
            v-if="
              !isEditing &&
              currentMember &&
              (link.approval_status === 'rejected' ||
                link.approval_status?.toLowerCase() === 'rejected')
            "
            class="btn btn-primary btn-small"
            @click.stop="openResubmitDialog(link)"
          >
            <UndoIcon aria-hidden="true" />
            重新提交
          </button>
          <!-- 消息按钮（仅项目成员在非编辑模式下显示） -->
          <button
            v-if="!isEditing && currentMember"
            class="btn btn-secondary btn-small message-toggle"
            @click.stop="toggleThread(link)"
          >
            <MessageIcon aria-hidden="true" />
            {{ expandedThreads.includes(getLinkId(link)) ? "隐藏" : "显示" }}消息
          </button>
        </div>

        <!-- Thread 消息区域（仅项目成员可见） -->
        <div
          v-show="!isEditing && currentMember && expandedThreads.includes(getLinkId(link))"
          class="thread-section"
          @click.stop
        >
          <div class="thread-header">
            <h5>审核消息</h5>
            <span class="thread-description">与审核者的对话记录</span>
          </div>
          <div v-if="threads[getLinkId(link)]" class="thread-messages">
            <div
              v-if="
                threads[getLinkId(link)].messages && threads[getLinkId(link)].messages.length > 0
              "
              class="messages-list"
            >
              <div
                v-for="message in threads[getLinkId(link)].messages"
                :key="message.id"
                class="message-item"
                :class="{
                  'mod-message':
                    message.author_id &&
                    isStaff(getMessageAuthor(message, threads[getLinkId(link)])),
                }"
              >
                <div class="message-header">
                  <div class="message-author">
                    <Avatar
                      v-if="getMessageAuthor(message, threads[getLinkId(link)])"
                      :src="getMessageAuthor(message, threads[getLinkId(link)]).avatar_url"
                      :alt="getMessageAuthor(message, threads[getLinkId(link)]).username"
                      size="xs"
                      circle
                    />
                    <span>{{
                      getMessageAuthor(message, threads[getLinkId(link)])?.username || "系统"
                    }}</span>
                  </div>
                  <span class="message-time">{{ fromNow(message.created) }}</span>
                </div>
                <div class="message-body">
                  <template v-if="message.body.type === 'text'">
                    <div class="message-text" v-html="renderMarkdown(message.body.body)"></div>
                  </template>
                  <template v-else-if="message.body.type === 'status_change'">
                    <div class="status-change">
                      状态变更: {{ formatApprovalStatus(message.body.old_status) }} →
                      {{ formatApprovalStatus(message.body.new_status) }}
                    </div>
                  </template>
                </div>
              </div>
            </div>
            <div v-else class="no-messages">
              <InfoIcon aria-hidden="true" />
              <p>{{ threads[getLinkId(link)].noThreadMessage || "暂无消息记录" }}</p>
            </div>

            <!-- 发送消息区域（仅当前用户是项目成员时显示） -->
            <div v-if="currentMember" class="send-message">
              <textarea
                v-model="messageTexts[getLinkId(link)]"
                class="message-input"
                placeholder="输入消息..."
                rows="3"
                @click.stop
              ></textarea>
              <div class="message-actions">
                <button
                  class="btn btn-primary btn-small"
                  :disabled="!messageTexts[getLinkId(link)] || sendingMessage[getLinkId(link)]"
                  @click.stop="sendMessage(link)"
                >
                  <SendIcon aria-hidden="true" />
                  发送
                </button>
              </div>
            </div>
          </div>
          <!-- 如果没有thread，显示空消息界面而不是加载中 -->
          <div v-else class="thread-messages">
            <div class="no-messages">
              <InfoIcon aria-hidden="true" />
              <p>暂无消息记录</p>
            </div>

            <!-- 发送消息区域（即使没有thread也显示，第一条消息会创建thread） -->
            <div v-if="currentMember" class="send-message">
              <textarea
                v-model="messageTexts[getLinkId(link)]"
                class="message-input"
                placeholder="输入消息..."
                rows="3"
                @click.stop
              ></textarea>
              <div class="message-actions">
                <button
                  class="btn btn-primary btn-small"
                  :disabled="!messageTexts[getLinkId(link)] || sendingMessage[getLinkId(link)]"
                  @click.stop="sendMessage(link)"
                >
                  <SendIcon aria-hidden="true" />
                  发送
                </button>
              </div>
            </div>
          </div>
        </div>
      </template>
      <!-- 加载指示器 -->
      <div v-if="!isEditing && versionLinksLoading" class="loading-indicator">
        <span>正在加载版本链接信息...</span>
      </div>

      <div v-if="isEditing && versionLinks.length > 0" class="version-link-notice">
        <div class="notice-box">
          <InfoIcon aria-hidden="true" />
          <span>一个翻译版本只能绑定一个原版本。如需更改，请先移除当前绑定。</span>
        </div>
      </div>

      <div v-if="isEditing && versionLinks.length === 0" class="add-version-link">
        <!-- 审核说明 -->
        <div class="approval-info-box">
          <InfoIcon aria-hidden="true" />
          <div class="approval-info-content">
            <p><strong>版本链接审核说明：</strong></p>
            <p>以下情况将<span class="highlight-green">自动通过审核</span>，无需等待：</p>
            <ul>
              <li>您是<strong>超级管理员</strong>或<strong>社区管理员</strong></li>
              <li>您是<strong>目标项目的团队成员</strong>（拥有上传版本权限）</li>
              <li>您是<strong>目标项目所属组织的成员</strong>（拥有上传版本权限）</li>
            </ul>
            <p>
              其他情况需要<span class="highlight-orange">等待审核</span
              >。审核将由<strong>目标项目的团队成员</strong>（拥有审核权限）或<strong>超级管理员/社区管理员</strong>进行处理，通过后您的翻译才会在目标版本显示。
            </p>
          </div>
        </div>
        <h4>绑定原版本（被翻译的版本）</h4>
        <div class="input-group">
          <input
            v-model="linkTargetId"
            type="text"
            placeholder="请输入版本 ID"
            @keyup.enter="addVersionLink('version', linkTargetId)"
          />
        </div>

        <div class="input-group">
          <label for="link-language-code">
            <span class="label__title">翻译语言</span>
          </label>
          <Multiselect
            id="link-language-code"
            v-model="linkLanguageCode"
            :options="[
              { value: 'zh_CN', label: '简体中文' },
              { value: 'zh_TW', label: '繁体中文' },
              { value: 'en_US', label: '英语' },
              { value: 'ja_JP', label: '日语' },
              { value: 'ko_KR', label: '韩语' },
            ]"
            :custom-label="(o) => (o ? o.label : '')"
            track-by="value"
            placeholder="选择翻译语言..."
            :searchable="false"
            :close-on-select="true"
            :show-labels="false"
          />
        </div>

        <div class="input-group">
          <label for="link-description">
            <span class="label__title">说明（可选）</span>
          </label>
          <input
            id="link-description"
            v-model="linkDescription"
            type="text"
            placeholder="翻译说明..."
          />
        </div>

        <div class="input-group">
          <ButtonStyled color="brand">
            <button @click="addVersionLink('version', linkTargetId)">
              <PlusIcon aria-hidden="true" />
              添加绑定
            </button>
          </ButtonStyled>
        </div>
      </div>
    </div>
    <div
      v-if="
        isEditing ||
        isCreating ||
        version.files.filter(
          (x) => x.url.includes('cdn.bbsmc.net') || x.url.startsWith('private://'),
        ).length > 0
      "
      class="version-page__files universal-card"
    >
      <h3>文件</h3>
      <!--      编辑中-->
      <div v-if="isEditing && replaceFile" class="file primary">
        <FileIcon aria-hidden="true" />
        <span class="filename">
          <strong>{{ replaceFile.name }}</strong>
          <span class="file-size">({{ $formatBytes(replaceFile.size) }})</span>
        </span>
        <FileInput
          class="iconified-button raised-button"
          prompt="替换"
          aria-label="替换"
          :accept="acceptFileFromProjectType(project.project_type)"
          :max-size="1073741824"
          should-always-reset
          @change="(x) => (replaceFile = x[0])"
        >
          <TransferIcon aria-hidden="true" />
        </FileInput>
      </div>
      <!--      非编辑-->
      <div
        v-for="(file, index) in version.files.filter(
          (x) => x.url.includes('cdn.bbsmc.net') || x.url.startsWith('private://'),
        )"
        :key="file.hashes.sha1"
        :class="{
          file: true,
          primary: primaryFile.hashes.sha1 === file.hashes.sha1,
        }"
      >
        <!--        <div v-if="file.url.includes('cdn.bbsmc.net')">-->
        <!--          -->
        <!--        </div>-->
        <FileIcon aria-hidden="true" />
        <span class="filename">
          <strong>{{ file.filename }}</strong>
          <span class="file-size">({{ $formatBytes(file.size) }})</span>
          <span v-if="primaryFile.hashes.sha1 === file.hashes.sha1" class="file-type"> 主要 </span>
          <span
            v-else-if="file.file_type === 'required-resource-pack' && !isEditing"
            class="file-type"
          >
            必选资源包
          </span>
          <span
            v-else-if="file.file_type === 'optional-resource-pack' && !isEditing"
            class="file-type"
          >
            可选资源包
          </span>
        </span>
        <multiselect
          v-if="
            version.loaders.some((x) => tags.loaderData.dataPackLoaders.includes(x)) &&
            isEditing &&
            primaryFile.hashes.sha1 !== file.hashes.sha1
          "
          v-model="oldFileTypes[index]"
          class="raised-multiselect"
          placeholder="选择文件的类型"
          :options="fileTypes"
          track-by="value"
          label="display"
          :searchable="false"
          :close-on-select="true"
          :show-labels="false"
          :allow-empty="false"
        />
        <ButtonStyled v-if="isEditing">
          <button
            :disabled="version.files.length + newFiles.length - 1 === deleteFiles.length"
            @click="
              () => {
                deleteFiles.push(file.hashes.sha1);
                version.files.splice(index, 1);
                oldFileTypes.splice(index, 1);
              }
            "
          >
            <TrashIcon aria-hidden="true" />
            删除
          </button>
        </ButtonStyled>
        <ButtonStyled v-else>
          <!-- CDN 文件直接下载 -->
          <a
            v-if="file.url.includes('cdn.bbsmc.net')"
            :href="file.url"
            class="raised-button"
            :title="`Download ${file.filename}`"
            tabindex="0"
          >
            <DownloadIcon aria-hidden="true" />
          </a>

          <!-- 私有文件通过 API 获取下载链接 -->
          <a
            v-else-if="isPrivateUrl(file.url)"
            :href="privateDownload.getHref(file)"
            class="raised-button"
            :class="{ 'cursor-wait': privateDownload.isDownloading.value }"
            :title="`Download ${file.filename}`"
            tabindex="0"
            @click="privateDownload.getDownloadHandler(file)($event)"
          >
            <span v-if="privateDownload.isDownloading.value" class="animate-spin">...</span>
            <DownloadIcon v-else aria-hidden="true" />
            付费下载
          </a>

          <!-- 外部文件在新标签页打开 -->
          <a
            v-else
            :href="file.url"
            class="raised-button"
            :title="`Download ${file.filename}`"
            target="_blank"
            tabindex="0"
          >
            <DownloadIcon aria-hidden="true" />
            下载
          </a>
        </ButtonStyled>
      </div>
      <template v-if="isEditing">
        <div v-for="(file, index) in newFiles" :key="index" class="file">
          <FileIcon aria-hidden="true" />
          <span class="filename">
            <strong>{{ file.name }}</strong>
            <span class="file-size">({{ $formatBytes(file.size) }})</span>
          </span>
          <multiselect
            v-if="version.loaders.some((x) => tags.loaderData.dataPackLoaders.includes(x))"
            v-model="newFileTypes[index]"
            class="raised-multiselect"
            placeholder="选择文件类型"
            :options="fileTypes"
            track-by="value"
            label="display"
            :searchable="false"
            :close-on-select="true"
            :show-labels="false"
            :allow-empty="false"
          />
          <ButtonStyled>
            <button
              class="raised-button"
              @click="
                () => {
                  newFiles.splice(index, 1);
                  newFileTypes.splice(index, 1);
                }
              "
            >
              <TrashIcon aria-hidden="true" />
              删除
            </button>
          </ButtonStyled>
        </div>
        <div class="additional-files">
          <h4>上传更多文件</h4>
          <span v-if="version.loaders.some((x) => tags.loaderData.dataPackLoaders.includes(x))">
            可继续上传文档,等其他依赖文件
          </span>
          <span v-else>用于源文件或 使用文档 等文件。</span>
          <FileInput
            prompt="拖放即可上传或单击即可选择"
            aria-label="上传附加文件"
            multiple
            long-style
            :accept="acceptFileFromProjectType(project.project_type)"
            :max-size="1073741824"
            @change="
              (x) =>
                x.forEach((y) => {
                  newFiles.push(y);
                  newFileTypes.push(null);
                })
            "
          >
            <UploadIcon aria-hidden="true" />
          </FileInput>
        </div>
      </template>
    </div>
    <div class="version-page__metadata">
      <div class="universal-card full-width-inputs">
        <h3>更多信息</h3>
        <div>
          <h4 style="margin-bottom: 10px">资源类型</h4>
          <Multiselect
            v-if="isEditing"
            v-model="version.type"
            class="input"
            placeholder="选择"
            :options="['software', 'minecraft', 'language']"
            :custom-label="
              (value) =>
                value === 'software'
                  ? '软件资源'
                  : value === 'minecraft'
                    ? '我的世界资源'
                    : '汉化包'
            "
            :searchable="false"
            :close-on-select="true"
            :show-labels="false"
            :allow-empty="false"
            @select="
              (newValue) => {
                if (newValue === 'language') {
                  version.disk_only = false;
                }
              }
            "
          />
          <template v-else>
            <span v-if="version.type === 'software'">软件资源</span>
            <span v-else-if="version.type === 'minecraft'">Minecraft资源</span>
            <span v-else-if="version.type === 'language'">汉化包</span>
          </template>
        </div>
        <div>
          <h4>发布版本</h4>
          <Multiselect
            v-if="isEditing"
            v-model="version.version_type"
            class="input"
            placeholder="选择"
            :options="['release', 'beta', 'alpha']"
            :custom-label="(value) => formatProjectRelease()(value)"
            :searchable="false"
            :close-on-select="true"
            :show-labels="false"
            :allow-empty="false"
          />
          <template v-else>
            <Badge
              v-if="version.version_type === 'release'"
              class="value"
              type="release"
              color="green"
            />
            <Badge
              v-else-if="version.version_type === 'beta'"
              class="value"
              type="beta"
              color="orange"
            />
            <Badge
              v-else-if="version.version_type === 'alpha'"
              class="value"
              type="alpha"
              color="red"
            />
          </template>
        </div>
        <div>
          <h4>版本号</h4>
          <div v-if="isEditing" class="iconified-input">
            <label class="hidden" for="version-number">Version number</label>
            <HashIcon aria-hidden="true" />
            <input
              id="version-number"
              v-model="version.version_number"
              type="text"
              autocomplete="off"
              maxlength="54"
            />
          </div>
          <span v-else>{{ version.version_number }}</span>
        </div>
        <div v-if="project.project_type !== 'resourcepack' && version.type !== 'language'">
          <h4>运行环境</h4>
          <Multiselect
            v-if="isEditing"
            v-model="version.loaders"
            :options="
              tags.loaders
                .filter((x) =>
                  x.supported_project_types.includes(project.actualProjectType.toLowerCase()),
                )
                .map((it) => it.name)
            "
            :custom-label="(value) => $formatCategory(value)"
            :loading="tags.loaders.length === 0"
            :multiple="true"
            :searchable="true"
            :show-no-results="false"
            :close-on-select="false"
            :clear-on-select="false"
            :show-labels="false"
            :limit="6"
            :hide-selected="true"
            placeholder="请选择一个运行环境..."
          />
          <Categories v-else :categories="version.loaders" :type="project.actualProjectType" />
        </div>
        <div v-if="version.type === 'minecraft'">
          <h4>游戏版本</h4>

          <template v-if="isEditing">
            <multiselect
              v-model="version.game_versions"
              :options="
                showSnapshots
                  ? tags.gameVersions.map((x) => x.version)
                  : tags.gameVersions
                      .filter((it) => it.version_type === 'release')
                      .map((x) => x.version)
              "
              :loading="tags.gameVersions.length === 0"
              :multiple="true"
              :searchable="true"
              :show-no-results="false"
              :close-on-select="false"
              :clear-on-select="false"
              :show-labels="false"
              :limit="6"
              :hide-selected="true"
              :custom-label="(version) => version"
              placeholder="选择支持的MC版本"
            />
            <Checkbox
              v-model="showSnapshots"
              label="显示全部版本"
              description="显示全部版本"
              style="margin-top: 0.5rem"
              :border="false"
            />
          </template>
          <span v-else>{{ $formatVersion(version.game_versions) }}</span>
        </div>
        <div v-if="!isEditing">
          <h4>下载量</h4>
          <span>{{ version.downloads }}</span>
        </div>
        <div v-if="!isEditing">
          <h4>发布时间</h4>
          <span>
            {{ $dayjs(version.date_published).format("YYYY-MM-DD HH:mm:ss") }}
          </span>
        </div>
        <div v-if="!isEditing && version.author">
          <h4>创作者</h4>
          <div
            class="team-member columns button-transparent"
            @click="$router.push('/user/' + version.author.user.username)"
          >
            <Avatar
              :src="version.author.avatar_url"
              :alt="version.author.user.username"
              size="sm"
              circle
            />

            <div class="member-info">
              <nuxt-link :to="'/user/' + version.author.user.username" class="name">
                <p>
                  {{ version.author.name }}
                </p>
              </nuxt-link>
              <p v-if="version.author.role" class="role">
                {{ version.author.role }}
              </p>
              <p v-else-if="version.author_id === 'GVFjtWTf'" class="role">Archivist</p>
            </div>
          </div>
        </div>
        <div v-if="!isEditing">
          <h4>版本号</h4>
          <CopyCode :text="version.id" />
        </div>
      </div>
    </div>
  </div>
</template>
<script>
import { formatProjectRelease, renderString } from "@modrinth/utils";
import { ButtonStyled, ConfirmModal, MarkdownEditor, NewModal } from "@modrinth/ui";
import { Multiselect } from "vue-multiselect";
import JSZip from "jszip";
import UploadModal from "@modrinth/ui/src/components/modal/UploadModal.vue";
import ConfirmModal2 from "@modrinth/ui/src/components/modal/ConfirmModal2.vue";
import { acceptFileFromProjectType } from "~/helpers/fileUtils.js";
import { inferVersionInfo } from "~/helpers/infer.js";
import { renderHighlightedString } from "~/helpers/highlight.js";
import { reportVersion } from "~/utils/report-helpers.ts";
import { useImageUpload } from "~/composables/image-upload.ts";

import Avatar from "~/components/ui/Avatar.vue";
import Badge from "~/components/ui/Badge.vue";
import Breadcrumbs from "~/components/ui/Breadcrumbs.vue";
import CopyCode from "~/components/ui/CopyCode.vue";
import Categories from "~/components/ui/search/Categories.vue";
import Chips from "~/components/ui/Chips.vue";
import Checkbox from "~/components/ui/Checkbox.vue";
import FileInput from "~/components/ui/FileInput.vue";

import FileIcon from "~/assets/images/utils/file.svg?component";
import TrashIcon from "~/assets/images/utils/trash.svg?component";
import EditIcon from "~/assets/images/utils/edit.svg?component";
import DownloadIcon from "~/assets/images/utils/download.svg?component";
import StarIcon from "~/assets/images/utils/star.svg?component";
import ReportIcon from "~/assets/images/utils/report.svg?component";
import SaveIcon from "~/assets/images/utils/save.svg?component";
import CrossIcon from "~/assets/images/utils/x.svg?component";
import HashIcon from "~/assets/images/utils/hash.svg?component";
import PlusIcon from "~/assets/images/utils/plus.svg?component";
import TransferIcon from "~/assets/images/utils/transfer.svg?component";
import UploadIcon from "~/assets/images/utils/upload.svg?component";
import BackIcon from "~/assets/images/utils/left-arrow.svg?component";
import BoxIcon from "~/assets/images/utils/box.svg?component";
import RightArrowIcon from "~/assets/images/utils/right-arrow.svg?component";
import InfoIcon from "~/assets/images/utils/info.svg?component";
import UndoIcon from "~/assets/images/utils/undo.svg?component";
// SendIcon removed - unused
import Modal from "~/components/ui/Modal.vue";
import ChevronRightIcon from "~/assets/images/utils/chevron-right.svg?component";
import { useBaseFetchFile } from "~/composables/fetch.js";
import VersionSummary from "~/components/ui/VersionSummary.vue";
import AutomaticAccordion from "~/components/ui/AutomaticAccordion.vue";
import TranslationPromo from "~/components/ui/TranslationPromo.vue";
import { usePrivateDownload, isPrivateUrl } from "~/composables/usePrivateDownload.ts";

export default defineNuxtComponent({
  components: {
    ConfirmModal2,
    AutomaticAccordion,
    VersionSummary,
    NewModal,
    UploadModal,
    MarkdownEditor,
    Modal,
    FileInput,
    Checkbox,
    ChevronRightIcon,
    Chips,
    Categories,
    DownloadIcon,
    EditIcon,
    TrashIcon,
    StarIcon,
    FileIcon,
    ReportIcon,
    SaveIcon,
    CrossIcon,
    HashIcon,
    PlusIcon,
    TransferIcon,
    UploadIcon,
    BackIcon,
    Avatar,
    Badge,
    Breadcrumbs,
    CopyCode,
    Multiselect,
    BoxIcon,
    RightArrowIcon,
    InfoIcon,
    UndoIcon,
    ConfirmModal,
    ButtonStyled,
    TranslationPromo,
  },
  props: {
    project: {
      type: Object,
      default() {
        return {};
      },
    },
    versions: {
      type: Array,
      default() {
        return [];
      },
    },
    featuredVersions: {
      type: Array,
      default() {
        return [];
      },
    },
    members: {
      type: Array,
      default() {
        return [{}];
      },
    },
    currentMember: {
      type: Object,
      default() {
        return null;
      },
    },
    dependencies: {
      type: Object,
      default() {
        return {};
      },
    },
    resetProject: {
      type: Function,
      required: true,
      default: () => {},
    },
  },
  async setup(props) {
    const data = useNuxtApp();
    const route = useNativeRoute();

    const auth = await useAuth();
    const tags = useTags();
    const flags = useFeatureFlags();

    const path = route.name.split("-");
    const mode = path[path.length - 1];

    const fileTypes = [
      {
        display: "Required resource pack",
        value: "required-resource-pack",
      },
      {
        display: "Optional resource pack",
        value: "optional-resource-pack",
      },
    ];
    let oldFileTypes = [];

    let isCreating = false;
    const uploading = "";
    const uploadSpeed = "";
    let isEditing = false;

    let version = {};
    let primaryFile = {};
    let alternateFile = {};

    let replaceFile = null;

    // 提前声明 versionLinks 变量，供创建模式使用
    let versionLinks = [];
    let translationVersions = [];
    let versionLinksLoading = false;
    let translationVersionsLoading = false;

    // 重新提交相关变量（保留以备将来使用）
    // const resubmitReason = "";
    // const resubmittingLink = false;
    // const pendingResubmitLink = null;

    if (mode === "edit") {
      isEditing = true;
    }

    if (route.params.version === "create") {
      isCreating = true;
      isEditing = true;

      // 新建资源的版本信息
      version = {
        id: "none",
        project_id: props.project.id,
        author_id: props.currentMember.user.id,
        name: "",
        version_number: "",
        changelog: "",
        date_published: Date.now(),
        downloads: 0,
        version_type: "release",
        files: [],
        dependencies: [],
        game_versions: [],
        loaders: [],
        featured: false,
        quark_disk: "",
        baidu_disk: "",
        xunlei_disk: "",
        disk_only: false,
        curseforge: "",
        modrinth: "",
        is_modpack: false,
        type: undefined,
      };
      // 用于从版本页面导航/上传文件提示

      if (import.meta.client && history.state && history.state.newPrimaryFile) {
        replaceFile = history.state.newPrimaryFile;

        try {
          const inferredData = await inferVersionInfo(
            replaceFile,
            props.project,
            tags.value.gameVersions,
          );

          version = {
            ...version,
            ...inferredData,
          };

          // 如果推断数据包含目标版本信息，自动创建版本链接
          if (inferredData.targetVersion) {
            try {
              // 获取目标版本信息
              const targetVersionResponse = await useBaseFetch(
                `version/${inferredData.targetVersion}`,
              );

              if (targetVersionResponse && targetVersionResponse.project_id) {
                // 获取项目信息
                const projectResponse = await useBaseFetch(
                  `project/${targetVersionResponse.project_id}`,
                );

                if (projectResponse) {
                  const projectType =
                    projectResponse.project_type || projectResponse.project_types?.[0] || "mod";

                  versionLinks = [
                    {
                      joining_version_id: inferredData.targetVersion,
                      link_type: "translation",
                      language_code: inferredData.languageCode || "zh_CN",
                      description: inferredData.linkDescription || "",
                      originalVersion: {
                        ...targetVersionResponse,
                        project: projectResponse,
                        link: `/${projectType}/${projectResponse.slug || projectResponse.id}/version/${encodeURI(targetVersionResponse.version_number)}`,
                      },
                    },
                  ];
                }
              }
            } catch (error) {
              console.error("加载目标版本信息失败:", inferredData.targetVersion, error);
              // 即使加载失败，仍然创建基本的版本链接
              versionLinks = [
                {
                  joining_version_id: inferredData.targetVersion,
                  link_type: "translation",
                  language_code: inferredData.languageCode || "zh_CN",
                  description: inferredData.linkDescription || "",
                  originalVersion: null,
                },
              ];
            }

            // 设置版本类型为语言类型
            version.type = "language";

            // 删除临时字段，避免发送到后端
            delete version.targetVersion;
            delete version.languageCode;
            delete version.linkDescription;
          }
        } catch (err) {
          console.error("解析版本文件数据时出错", err);
        }
      } else {
        version.disk_only = true;
      }
      // 只在没有设置type的情况下才设为null
      if (version.type === undefined) {
        version.type = null;
      }
    } else if (route.params.version === "latest") {
      let versionList = props.versions;
      if (route.query.loader) {
        versionList = versionList.filter((x) => x.loaders.includes(route.query.loader));
      }
      if (route.query.version) {
        versionList = versionList.filter((x) => x.game_versions.includes(route.query.version));
      }
      // Upstream fix fd9653e28: Handle empty version list when filters match nothing
      if (versionList.length === 0) {
        throw createError({
          fatal: true,
          statusCode: 404,
          message: "No version matches the filters",
        });
      }
      version = versionList.reduce((a, b) => (a.date_published > b.date_published ? a : b));
    } else {
      version = props.versions.find((x) => x.id === route.params.version);

      if (!version) {
        version = props.versions.find((x) => x.displayUrlEnding === route.params.version);
      }
    }

    if (!version) {
      throw createError({
        fatal: true,
        statusCode: 404,
        message: "Version not found",
      });
    }

    version = JSON.parse(JSON.stringify(version));

    if (version.disk_only && version.disk_urls && version.disk_urls.length > 0) {
      version.disk_urls.forEach((url) => {
        if (url.platform === "baidu") {
          version.baidu_disk = url.url;
        } else if (url.platform === "xunlei") {
          version.xunlei_disk = url.url;
        } else if (url.platform === "quark") {
          version.quark_disk = url.url;
        } else if (url.platform === "modrinth") {
          version.modrinth = url.url;
        } else if (url.platform === "curseforge") {
          version.curseforge = url.url;
        }
      });
    }
    if (version.loaders.length > 0) {
      if (
        version.loaders.includes("windows") ||
        version.loaders.includes("linux") ||
        version.loaders.includes("macos")
      ) {
        version.type = "software";
      } else if (version.loaders.includes("language")) {
        version.type = "language";
      } else {
        version.type = "minecraft";
      }
    }

    primaryFile = version.files.find((file) => file.primary) ?? version.files[0];

    alternateFile = version.files.find(
      (file) => file.file_type && file.file_type.includes("resource-pack"),
    );

    for (const dependency of version.dependencies) {
      dependency.version = props.dependencies.versions.find((x) => x.id === dependency.version_id);

      if (dependency.version) {
        dependency.project = props.dependencies.projects.find(
          (x) => x.id === dependency.version.project_id,
        );
      }

      if (!dependency.project) {
        dependency.project = props.dependencies.projects.find(
          (x) => x.id === dependency.project_id,
        );
      }

      dependency.link = dependency.project
        ? `/${dependency.project.project_type}/${dependency.project.slug ?? dependency.project.id}${
            dependency.version ? `/version/${encodeURI(dependency.version.version_number)}` : ""
          }`
        : "";
    }

    // 初始化版本链接数据（与依赖项目初始化方式一致）
    // 注意：versionLinks等变量已在前面声明，这里只处理非创建模式的情况
    if (!isCreating && version.version_links && version.version_links.length > 0) {
      versionLinksLoading = true;
      versionLinks = await Promise.all(
        version.version_links.map(async (link) => {
          try {
            const versionResponse = await useBaseFetch(`version/${link.joining_version_id}`);

            if (versionResponse && versionResponse.project_id) {
              const projectResponse = await useBaseFetch(`project/${versionResponse.project_id}`);

              if (projectResponse) {
                const projectType =
                  projectResponse.project_type || projectResponse.project_types?.[0] || "mod";

                return {
                  ...link,
                  originalVersion: {
                    ...versionResponse,
                    project: projectResponse,
                    link: `/${projectType}/${projectResponse.slug || projectResponse.id}/version/${encodeURI(versionResponse.version_number)}`,
                  },
                };
              }
            }
          } catch (error) {
            console.error("加载版本链接详情失败:", link.joining_version_id, error);
          }

          return {
            ...link,
            originalVersion: null,
          };
        }),
      );
      versionLinksLoading = false;
    }

    // 初始化翻译版本数据（与依赖项目初始化方式一致）
    if (version.translated_by && version.translated_by.length > 0) {
      translationVersionsLoading = true;

      // 获取所有翻译版本的详细信息
      const allTranslations = await Promise.all(
        version.translated_by.map(async (link) => {
          try {
            const versionResponse = await useBaseFetch(`version/${link.joining_version_id}`);

            if (versionResponse && versionResponse.project_id) {
              const projectResponse = await useBaseFetch(`project/${versionResponse.project_id}`);

              if (projectResponse) {
                const projectType =
                  projectResponse.project_type || projectResponse.project_types?.[0] || "mod";

                return {
                  ...link,
                  project: projectResponse,
                  version: versionResponse,
                  version_number: versionResponse.version_number,
                  date_published: versionResponse.date_published,
                  link: `/${projectType}/${projectResponse.slug || projectResponse.id}/version/${encodeURI(versionResponse.version_number)}`,
                };
              }
            }
          } catch (error) {
            console.error("加载翻译版本详情失败:", link.joining_version_id, error);
          }

          return null;
        }),
      );

      // 过滤掉加载失败的版本
      const validTranslations = allTranslations.filter((t) => t && t.project);

      // 按项目分组，每个项目只保留最新的版本
      const projectMap = new Map();

      for (const translation of validTranslations) {
        const projectId = translation.project.id;

        if (!projectMap.has(projectId)) {
          projectMap.set(projectId, translation);
        } else {
          const existing = projectMap.get(projectId);
          // 比较发布日期，保留更新的版本
          if (new Date(translation.date_published) > new Date(existing.date_published)) {
            projectMap.set(projectId, translation);
          }
        }
      }

      // 转换为数组并按发布日期降序排序
      translationVersions = Array.from(projectMap.values()).sort(
        (a, b) => new Date(b.date_published) - new Date(a.date_published),
      );

      translationVersionsLoading = false;
    }

    oldFileTypes = version.files.map((x) => fileTypes.find((y) => y.value === x.file_type));

    const title = computed(
      () =>
        `${isCreating ? "创建版本" : version.name} - ${props.project.title} | BBSMC 我的世界资源下载`,
    );
    const description = computed(() => {
      const gameVersionStr = data.$formatVersion(version.game_versions);
      const loaderStr = version.loaders.length
        ? version.loaders.map((x) => x.charAt(0).toUpperCase() + x.slice(1)).join(" & ")
        : "";
      const supportStr = [gameVersionStr, loaderStr].filter(Boolean).join(" ");
      return `下载 ${props.project.title} ${version.version_number}。${supportStr ? `支持 ${supportStr}。` : ""}发布于 ${data.$dayjs(version.date_published).format("YYYY-MM-DD")}，已有 ${version.downloads} 次下载。在 BBSMC 获取最新版本。`;
    });

    useSeoMeta({
      title,
      description,
      ogTitle: title,
      ogDescription: description,
      ogImage: () => props.project.icon_url ?? "https://cdn.bbsmc.net/raw/placeholder.png",
      robots: isCreating ? "noindex, nofollow" : undefined,
    });

    // 私有文件下载支持
    const privateDownload = usePrivateDownload();

    return {
      auth,
      tags,
      flags,
      fileTypes: ref(fileTypes),
      oldFileTypes: ref(oldFileTypes),
      isCreating: ref(isCreating),
      uploading: ref(uploading),
      uploadSpeed: ref(uploadSpeed),
      isEditing: ref(isEditing),
      version: ref(version),
      primaryFile: ref(primaryFile),

      alternateFile: ref(alternateFile),
      replaceFile: ref(replaceFile),
      uploadedImageIds: ref([]),

      // 版本链接和翻译版本数据
      versionLinks: ref(versionLinks),
      versionLinksLoading: ref(versionLinksLoading),
      translationVersions: ref(translationVersions),
      translationVersionsLoading: ref(translationVersionsLoading),

      // 私有文件下载
      isPrivateUrl,
      privateDownload,
    };
  },
  data() {
    return {
      dependencyAddMode: "project",
      newDependencyType: "required",
      newDependencyId: "",

      showSnapshots: false,

      newFiles: [],
      deleteFiles: [],
      replaceFile: null,

      newFileTypes: [],

      packageLoaders: ["forge", "fabric", "quilt", "neoforge"],

      showKnownErrors: false,
      shouldPreventActions: false,

      // 版本链接编辑相关数据
      linkTargetId: "",
      linkLanguageCode: { value: "zh_CN", label: "简体中文" },
      linkDescription: "",

      // Thread相关数据
      expandedThreads: [],
      threads: {},
      messageTexts: {},
      sendingMessage: {},

      // 重新提交相关数据
      resubmitReason: "",
      resubmittingLink: false,
      pendingResubmitLink: null,
    };
  },
  computed: {
    fieldErrors() {
      return (
        this.version.version_number === "" ||
        (this.version.disk_only &&
          (this.version.quark_disk === "" || this.version.quark_disk === undefined) &&
          (this.version.xunlei_disk === "" || this.version.xunlei_disk === undefined) &&
          (this.version.modrinth === "" || this.version.modrinth === undefined) &&
          (this.version.curseforge === "" || this.version.curseforge === undefined) &&
          (this.version.baidu_disk === "" || this.version.baidu_disk === undefined)) ||
        (this.version.game_versions.length === 0 && this.version.type === "minecraft") ||
        (this.version.loaders.length === 0 &&
          this.project.project_type !== "resourcepack" &&
          this.version.type !== "language") ||
        (this.newFiles.length === 0 &&
          this.version.files.length === 0 &&
          !this.replaceFile &&
          this.version.disk_only === false)
      );
    },
    deps() {
      const order = ["required", "optional", "incompatible", "embedded"];
      return [...this.version.dependencies].sort(
        (a, b) => order.indexOf(a.dependency_type) - order.indexOf(b.dependency_type),
      );
    },
  },
  watch: {
    "$route.path"() {
      const path = this.$route.name.split("-");
      const mode = path[path.length - 1];

      this.isEditing = mode === "edit" || this.$route.params.version === "create";
    },
  },
  mounted() {
    // 数据已在setup中初始化，无需重复操作
  },
  methods: {
    // 导航到汉化包版本页面
    navigateToTranslation(translationData) {
      if (translationData && translationData.project && translationData.version) {
        const projectType = translationData.project.project_type;
        const projectId = translationData.project.slug || translationData.project.id;
        const versionId = translationData.version.version_number || translationData.version.id;
        this.$router.push(`/${projectType}/${projectId}/version/${encodeURI(versionId)}`);
        this.$refs.downloadModal.hide();
      }
    },
    // 打开重新提交对话框
    openResubmitDialog(link) {
      this.pendingResubmitLink = link;
      this.resubmitReason = "";
      this.$refs.resubmitModal.show();
    },
    // 确认重新提交
    async confirmResubmit() {
      if (!this.pendingResubmitLink || !this.resubmitReason || this.resubmittingLink) {
        return;
      }

      this.resubmittingLink = true;

      try {
        // 构建正确的版本ID
        const versionId = this.version.id; // 当前翻译版本ID
        const targetVersionId = this.pendingResubmitLink.joining_version_id; // 目标版本ID

        // 调用后端API重新提交审核
        await useBaseFetch(`version/${versionId}/link/${targetVersionId}/resubmit`, {
          method: "POST",
          body: {
            reason: this.resubmitReason,
          },
        });

        this.$notify({
          group: "main",
          title: "成功",
          text: "已重新提交审核，请等待审核结果",
          type: "success",
        });

        // 关闭对话框
        this.$refs.resubmitModal.hide();

        // 更新链接状态为pending
        const linkIndex = this.versionLinks.findIndex(
          (l) => l.joining_version_id === this.pendingResubmitLink.joining_version_id,
        );
        if (linkIndex !== -1) {
          this.versionLinks[linkIndex].approval_status = "pending";
        }

        // 清理状态
        this.pendingResubmitLink = null;
        this.resubmitReason = "";
      } catch (error) {
        console.error("重新提交失败:", error);
        this.$notify({
          group: "main",
          title: "错误",
          text: error.data?.description || "重新提交失败，请稍后重试",
          type: "error",
        });
      } finally {
        this.resubmittingLink = false;
      }
    },
    formatDateTime(date) {
      return this.$dayjs(date).format("YYYY-MM-DD HH:mm");
    },
    fromNow(date) {
      return this.$dayjs(date).fromNow();
    },
    getApprovalStatusLabel(status) {
      const normalizedStatus = (status || "").toLowerCase();
      if (normalizedStatus === "approved") {
        return "已审核";
      } else if (normalizedStatus === "rejected") {
        return "已拒绝";
      } else if (normalizedStatus === "pending") {
        return "审核中";
      }
      return status;
    },
    async loadVersionLinkDetails() {
      // 设置加载状态
      this.versionLinksLoading = true;

      for (let i = 0; i < this.versionLinks.length; i++) {
        const link = this.versionLinks[i];

        try {
          // 使用正确的 API 路径获取版本信息
          const versionResponse = await useBaseFetch(`version/${link.joining_version_id}`);

          if (versionResponse && versionResponse.project_id) {
            // 获取项目信息
            const projectResponse = await useBaseFetch(`project/${versionResponse.project_id}`);

            if (projectResponse) {
              // 处理项目类型（兼容 v2 和 v3 API）
              const projectType =
                projectResponse.project_type || projectResponse.project_types?.[0] || "mod";

              // 构造 originalVersion 对象，与 addVersionLink 方法保持一致
              const originalVersionData = {
                ...versionResponse,
                project: projectResponse,
                link: `/${projectType}/${projectResponse.slug || projectResponse.id}/version/${encodeURI(versionResponse.version_number)}`,
              };

              // 在 Vue 3 中，需要重新赋值整个数组元素以触发响应式更新
              this.versionLinks[i] = {
                ...this.versionLinks[i],
                originalVersion: originalVersionData,
              };
            } else {
              console.warn("找不到项目信息:", versionResponse.project_id);
            }
          } else {
            console.warn("版本信息不完整或不存在:", link.joining_version_id);
          }
        } catch (error) {
          console.error("加载版本链接详情失败:", link.joining_version_id, error);
        }
      }

      // 加载完成，关闭加载状态
      this.versionLinksLoading = false;

      // 强制触发视图更新
      this.$forceUpdate();
    },
    async loadTranslationVersionDetails() {
      // 设置加载状态
      this.translationVersionsLoading = true;

      for (let i = 0; i < this.translationVersions.length; i++) {
        const link = this.translationVersions[i];

        try {
          // 使用joining_version_id获取翻译版本的信息
          const versionResponse = await useBaseFetch(`version/${link.joining_version_id}`);

          if (versionResponse && versionResponse.project_id) {
            // 获取项目信息
            const projectResponse = await useBaseFetch(`project/${versionResponse.project_id}`);

            if (projectResponse) {
              // 处理项目类型（兼容 v2 和 v3 API）
              const projectType =
                projectResponse.project_type || projectResponse.project_types?.[0] || "mod";

              // 更新翻译版本数据
              this.translationVersions[i] = {
                ...this.translationVersions[i],
                project: projectResponse,
                version_number: versionResponse.version_number,
                link: `/${projectType}/${projectResponse.slug || projectResponse.id}/version/${encodeURI(versionResponse.version_number)}`,
              };
            }
          }
        } catch (error) {
          console.error("加载翻译版本详情失败:", link.joining_version_id, error);
        }
      }

      // 加载完成，关闭加载状态
      this.translationVersionsLoading = false;

      // 强制触发视图更新
      this.$forceUpdate();
    },
    formatProjectRelease() {
      return formatProjectRelease;
    },
    async onImageUpload(file) {
      const response = await useImageUpload(file, { context: "version" });

      this.uploadedImageIds.push(response.id);
      this.uploadedImageIds = this.uploadedImageIds.slice(-10);

      return response.url;
    },
    getPreviousLink() {
      if (this.$router.options.history.state.back) {
        if (this.$router.options.history.state.back.includes("/versions")) {
          return this.$router.options.history.state.back;
        }
      }
      return `/${this.project.project_type}/${
        this.project.slug ? this.project.slug : this.project.id
      }/versions`;
    },
    getPreviousLabel() {
      return this.$router.options.history.state.back &&
        this.$router.options.history.state.back.endsWith("/versions")
        ? "返回到版本页面"
        : "全部版本";
    },
    acceptFileFromProjectType,
    renderHighlightedString,
    async addDependency(dependencyAddMode, newDependencyId, newDependencyType, hideErrors) {
      try {
        if (dependencyAddMode === "project") {
          const project = await useBaseFetch(`project/${newDependencyId}`);

          if (this.version.dependencies.some((dep) => project.id === dep.project_id)) {
            this.$notify({
              group: "main",
              title: "依赖项已添加",
              text: "您不能两次添加相同的依赖项。",
              type: "error",
            });
          } else {
            this.version.dependencies.push({
              project,
              project_id: project.id,
              dependency_type: newDependencyType,
              link: `/${project.project_type}/${project.slug ?? project.id}`,
            });

            this.$emit("update:dependencies", {
              projects: this.dependencies.projects.concat([project]),
              versions: this.dependencies.versions,
            });
          }
        } else if (dependencyAddMode === "version") {
          const version = await useBaseFetch(`version/${this.newDependencyId}`);

          const project = await useBaseFetch(`project/${version.project_id}`);

          if (this.version.dependencies.some((dep) => version.id === dep.version_id)) {
            this.$notify({
              group: "main",
              title: "依赖项已添加",
              text: "您不能两次添加相同的依赖项。",
              type: "error",
            });
          } else {
            this.version.dependencies.push({
              version,
              project,
              version_id: version.id,
              project_id: project.id,
              dependency_type: this.newDependencyType,
              link: `/${project.project_type}/${project.slug ?? project.id}/version/${encodeURI(
                version.version_number,
              )}`,
            });

            this.$emit("update:dependencies", {
              projects: this.dependencies.projects.concat([project]),
              versions: this.dependencies.versions.concat([version]),
            });
          }
        }

        this.newDependencyId = "";
      } catch {
        if (!hideErrors) {
          this.$notify({
            group: "main",
            title: "无效的依赖项",
            text: "找不到指定的依赖项",
            type: "error",
          });
        }
      }
    },
    async saveEditedVersion() {
      startLoading();

      if (this.fieldErrors) {
        this.showKnownErrors = true;

        stopLoading();
        return;
      }

      try {
        if (this.newFiles.length > 0) {
          const formData = new FormData();
          const fileParts = this.newFiles.map((f, idx) => `${f.name}-${idx}`);

          formData.append(
            "data",
            JSON.stringify({
              file_types: this.newFileTypes.reduce(
                (acc, x, i) => ({
                  ...acc,
                  [fileParts[i]]: x ? x.value : null,
                }),
                {},
              ),
            }),
          );

          for (let i = 0; i < this.newFiles.length; i++) {
            formData.append(fileParts[i], new Blob([this.newFiles[i]]), this.newFiles[i].name);
          }

          // await useBaseFetch(`version/${this.version.id}/file`, {
          //   method: "POST",
          //   body: formData,
          //   headers: {
          //     "Content-Disposition": formData,
          //   },
          // });

          this.$refs.uploading_modal.show();

          await useBaseFetchFile(`version/${this.version.id}/file`, {
            method: "POST",
            body: formData,
            headers: {
              "Content-Disposition": formData,
            },

            onUploadProgress: (progress, uploadSpeed) => {
              this.uploading = progress;
              this.uploadSpeed = uploadSpeed;
            },

            onError: (error) => {
              this.$refs.uploading_modal.proceed();
              this.$notify({
                group: "main",
                title: `${error.error}`,
                text: `${error.description}`,
                type: "error",
              });
            },
          });
          // this.$refs.uploading_modal.proceed()
        }

        const disks = [];
        if (this.version.quark_disk !== "" && this.version.quark_disk !== undefined) {
          disks.push({
            platform: "quark",
            url: this.version.quark_disk,
          });
        }
        if (this.version.baidu_disk !== "" && this.version.baidu_disk !== undefined) {
          disks.push({
            platform: "baidu",
            url: this.version.baidu_disk,
          });
        }
        if (this.version.curseforge !== "" && this.version.curseforge !== undefined) {
          disks.push({
            platform: "curseforge",
            url: this.version.curseforge,
          });
        }
        if (this.version.modrinth !== "" && this.version.modrinth !== undefined) {
          disks.push({
            platform: "modrinth",
            url: this.version.modrinth,
          });
        }
        if (this.version.xunlei_disk !== "" && this.version.xunlei_disk !== undefined) {
          disks.push({
            platform: "xunlei",
            url: this.version.xunlei_disk,
          });
        }
        // 如果版本类型是language，强制设置loaders为language
        const loaders = this.version.type === "language" ? ["language"] : this.version.loaders;

        const body = {
          name: this.version.name || this.version.version_number,
          version_number: this.version.version_number,
          changelog: this.version.changelog,
          version_type: this.version.version_type,
          dependencies: this.version.dependencies,

          loaders,
          disk_urls: this.version.disk_only ? disks : null,
          disk_only: this.version.disk_only,
          primary_file: this.version.disk_only ? [] : ["sha1", this.primaryFile.hashes.sha1],
          featured: this.version.featured,
          file_types: this.version.disk_only
            ? []
            : this.oldFileTypes.map((x, i) => {
                return {
                  algorithm: "sha1",
                  hash: this.version.files[i].hashes.sha1,
                  file_type: x ? x.value : null,
                };
              }),
        };

        // 语言类型版本不需要game_versions字段
        if (this.version.type === "language") {
          // 添加版本链接数据
          if (this.versionLinks.length > 0) {
            body.version_links = this.versionLinks.map((link) => ({
              joining_version_id: link.joining_version_id,
              link_type: link.link_type || "translation",
              language_code: link.language_code,
              description: link.description,
            }));
          } else {
            // 如果清空了所有链接，也需要更新
            body.version_links = [];
          }
        } else if (this.version.game_versions) {
          // 非语言类型版本才需要game_versions字段
          body.game_versions = this.version.game_versions;
        }

        if (this.project.project_type === "modpack") {
          delete body.dependencies;
        }

        await useBaseFetch(`version/${this.version.id}`, {
          method: "PATCH",
          body,
        });

        for (const hash of this.deleteFiles) {
          await useBaseFetch(`version_file/${hash}?version_id=${this.version.id}`, {
            method: "DELETE",
          });
        }

        await this.resetProjectVersions();

        await this.$router.push(
          `/${this.project.project_type}/${
            this.project.slug ? this.project.slug : this.project.id
          }/version/${this.version.id}`,
        );
      } catch (err) {
        console.log(err);
        this.$notify({
          group: "main",
          title: "发生错误",
          text: err.data.description,
          type: "error",
        });
        window.scrollTo({ top: 0, behavior: "smooth" });
      }
      stopLoading();
    },
    reportVersion,
    async createVersion() {
      this.shouldPreventActions = true;

      startLoading();
      if (this.fieldErrors) {
        console.log("fieldErrors");
        this.showKnownErrors = true;
        this.shouldPreventActions = false;

        stopLoading();
        return;
      }

      try {
        await this.createVersionRaw(this.version);
      } catch (err) {
        this.$notify({
          group: "main",
          title: "发生错误",
          text: err.data ? err.data.description : err,
          type: "error",
        });
        window.scrollTo({ top: 0, behavior: "smooth" });
      }

      stopLoading();
      this.shouldPreventActions = false;
    },
    async createVersionRaw(version) {
      const formData = new FormData();

      const fileParts = this.newFiles.map((f, idx) => `${f.name}-${idx}`);
      if (this.replaceFile) {
        fileParts.unshift(this.replaceFile.name.concat("-primary"));
      }

      if (this.project.project_type === "resourcepack") {
        version.loaders = ["minecraft"];
      }
      if (this.project.project_type === "language") {
        version.loaders = ["language"];
      }
      // 如果版本类型是language，强制设置loaders为language
      if (version.type === "language") {
        version.loaders = ["language"];
      }
      let curse = false;
      for (let i = 0; i < this.newFiles.length; i++) {
        const part = new Blob([this.newFiles[i]]);
        const zipReader = new JSZip();
        const zip = await zipReader.loadAsync(part);
        if (zip.file("manifest.json") || zip.file("modrinth.index.json")) {
          curse = true;
        }
      }

      if (this.replaceFile) {
        const part = new Blob([this.replaceFile]);
        const zipReader = new JSZip();
        const zip = await zipReader.loadAsync(part);
        if (zip.file("manifest.json") || zip.file("modrinth.index.json")) {
          curse = true;
        }
      }

      if (!curse && version.is_modpack) {
        curse = true;
      }
      const disks = [];
      if (version.quark_disk !== "" && version.quark_disk !== undefined) {
        disks.push({
          platform: "quark",
          url: version.quark_disk,
        });
      }
      if (version.baidu_disk !== "" && version.baidu_disk !== undefined) {
        disks.push({
          platform: "baidu",
          url: version.baidu_disk,
        });
      }
      if (version.curseforge !== "" && version.curseforge !== undefined) {
        disks.push({
          platform: "curseforge",
          url: version.curseforge,
        });
      }
      if (version.modrinth !== "" && version.modrinth !== undefined) {
        disks.push({
          platform: "modrinth",
          url: version.modrinth,
        });
      }
      if (version.xunlei_disk !== "" && version.xunlei_disk !== undefined) {
        disks.push({
          platform: "xunlei",
          url: version.xunlei_disk,
        });
      }
      const newVersion = {
        project_id: version.project_id,
        curse,
        software: version.type === "software",
        language: version.type === "language",
        file_parts: fileParts,
        version_number: version.version_number,
        version_title: version.name || version.version_number,
        version_body: version.changelog,
        dependencies: version.dependencies,
        loaders: version.loaders,
        release_channel: version.version_type,
        featured: version.featured,
        disk_only: version.disk_only,
        disk_urls: version.disk_only ? disks : null,
        primary_file: version.disk_only
          ? null
          : this.replaceFile
            ? this.replaceFile.name.concat("-primary")
            : fileParts[0],
        file_types:
          version.disk_only && this.newFileTypes.length === 0
            ? {}
            : this.newFileTypes.reduce(
                (acc, x, i) => ({
                  ...acc,
                  [fileParts[this.replaceFile ? i + 1 : i]]: x ? x.value : null,
                }),
                {},
              ),
      };
      if (this.version.type === "minecraft") {
        newVersion.game_versions = this.version.game_versions;
      }

      // 添加版本链接数据（仅限语言类型项目）
      if (this.version.type === "language" && this.versionLinks.length > 0) {
        newVersion.version_links = this.versionLinks.map((link) => ({
          joining_version_id: link.joining_version_id,
          link_type: link.link_type || "translation",
          language_code: link.language_code,
          description: link.description,
        }));
      }

      formData.append("data", JSON.stringify(newVersion));

      if (this.replaceFile) {
        formData.append(
          this.replaceFile.name.concat("-primary"),
          new Blob([this.replaceFile]),
          this.replaceFile.name,
        );
      }

      for (let i = 0; i < this.newFiles.length; i++) {
        formData.append(
          fileParts[this.replaceFile ? i + 1 : i],
          new Blob([this.newFiles[i]]),
          this.newFiles[i].name,
        );
      }
      this.$refs.uploading_modal.show();

      const data = await useBaseFetchFile("version", {
        method: "POST",
        body: formData,
        headers: {
          "Content-Disposition": formData,
        },

        onUploadProgress: (progress, uploadSpeed) => {
          this.uploading = progress;
          this.uploadSpeed = uploadSpeed;
        },

        onError: (error) => {
          this.$refs.uploading_modal.proceed();
          this.$notify({
            group: "main",
            title: `${error.error}`,
            text: `${error.description}`,
            type: "error",
          });
        },
      });
      await this.resetProjectVersions();

      await this.$router.push(
        `/${this.project.project_type}/${
          this.project.slug ? this.project.slug : this.project.project_id
        }/version/${data.id}`,
      );
    },
    async deleteVersion() {
      startLoading();

      await useBaseFetch(`version/${this.version.id}`, {
        method: "DELETE",
      });

      await this.resetProjectVersions();
      await this.$router.replace(`/${this.project.project_type}/${this.project.id}/versions`);
      stopLoading();
    },
    async resetProjectVersions() {
      const [versions, featuredVersions, dependencies] = await Promise.all([
        useBaseFetch(`project/${this.version.project_id}/version`),
        useBaseFetch(`project/${this.version.project_id}/version?featured=true`),
        useBaseFetch(`project/${this.version.project_id}/dependencies`),
        this.resetProject(),
      ]);

      const newCreatedVersions = this.$computeVersions(versions, this.members);
      const featuredIds = featuredVersions.map((x) => x.id);
      this.$emit("update:versions", newCreatedVersions);
      this.$emit(
        "update:featuredVersions",
        newCreatedVersions.filter((version) => featuredIds.includes(version.id)),
      );
      this.$emit("update:dependencies", dependencies);

      return newCreatedVersions;
    },

    // 版本链接相关方法

    async addVersionLink(_mode, targetId) {
      // 如果没有传入targetId，使用组件的状态
      if (!targetId) {
        targetId = this.linkTargetId;
      }

      if (!targetId) {
        this.$notify({
          group: "main",
          title: "缺少必要信息",
          text: "请输入版本 ID",
          type: "error",
        });
        return;
      }

      if (!this.linkLanguageCode || !this.linkLanguageCode.value) {
        this.$notify({
          group: "main",
          title: "缺少必要信息",
          text: "请选择翻译语言",
          type: "error",
        });
        return;
      }

      // 检查是否已经绑定了版本（只能绑定一个）
      if (this.versionLinks.length > 0) {
        this.$notify({
          group: "main",
          title: "绑定限制",
          text: "一个翻译版本只能绑定一个原版本，请先移除现有绑定",
          type: "error",
        });
        return;
      }

      try {
        // 直接通过版本ID添加
        const targetVersion = await useBaseFetch(`version/${targetId}`);

        if (!targetVersion || !targetVersion.project_id) {
          this.$notify({
            group: "main",
            title: "版本无效",
            text: "找不到指定的版本或版本数据不完整",
            type: "error",
          });
          return;
        }

        const targetProject = await useBaseFetch(`project/${targetVersion.project_id}`);

        if (!targetProject) {
          this.$notify({
            group: "main",
            title: "项目无效",
            text: "找不到版本对应的项目",
            type: "error",
          });
          return;
        }

        // 检查是否已存在相同的链接
        const exists = this.versionLinks.some(
          (link) => link.joining_version_id === targetVersion.id,
        );

        if (exists) {
          this.$notify({
            group: "main",
            title: "链接已存在",
            text: "该版本已经被链接",
            type: "error",
          });
          return;
        }

        // 处理项目类型（兼容 v2 和 v3 API）
        const projectType = targetProject.project_type || targetProject.project_types?.[0] || "mod";

        // 添加到本地列表
        this.versionLinks.push({
          joining_version_id: targetVersion.id,
          link_type: "translation",
          language_code: this.linkLanguageCode.value,
          description: this.linkDescription,
          // 用于显示的额外信息
          originalVersion: {
            project: targetProject,
            ...targetVersion,
            link: `/${projectType}/${targetProject.slug || targetProject.id}/version/${encodeURI(targetVersion.version_number)}`,
          },
        });

        // 如果不是创建模式，也更新 version 对象
        if (!this.isCreating && this.version.version_links) {
          this.version.version_links.push({
            joining_version_id: targetVersion.id,
            link_type: "translation",
            language_code: this.linkLanguageCode.value,
            description: this.linkDescription,
          });
        }

        // 清空输入
        this.linkTargetId = "";
        this.linkDescription = "";

        this.$notify({
          group: "main",
          title: "添加成功",
          text: `已绑定版本: ${targetVersion.name} (${targetVersion.version_number})`,
          type: "success",
        });
      } catch (error) {
        this.$notify({
          group: "main",
          title: "添加失败",
          text: "找不到指定的版本",
          type: "error",
        });
      }
    },

    removeVersionLink(index) {
      this.versionLinks.splice(index, 1);

      // 如果不是编辑模式，也更新 version 对象
      if (!this.isCreating && this.version.version_links) {
        this.version.version_links.splice(index, 1);
      }

      this.$notify({
        group: "main",
        title: "删除成功",
        text: "版本链接已删除",
        type: "success",
      });
    },

    onDownload(version) {
      useBaseFetch(`version/${version}/download`, {
        method: "PATCH",
        apiVersion: 3,
      });
    },

    // Thread相关方法
    toggleThread(link) {
      const linkId = this.getLinkId(link);
      if (!linkId) return;

      const index = this.expandedThreads.indexOf(linkId);
      if (index === -1) {
        this.expandedThreads.push(linkId);
        // 如果有thread_id且还没有获取过，尝试获取thread
        if (link.thread_id && !this.threads[linkId]) {
          this.fetchThread(link);
        }
        // 如果没有thread_id，不需要创建虚拟thread，因为模板会自动显示空消息界面
      } else {
        this.expandedThreads.splice(index, 1);
      }
      // 强制更新视图
      this.$forceUpdate();
    },

    async fetchThread(link) {
      if (!link || !link.thread_id) return;

      const linkId = this.getLinkId(link);
      if (!linkId) return;

      try {
        const thread = await useBaseFetch(`thread/${link.thread_id}`);
        this.threads[linkId] = thread;
      } catch (error) {
        console.error("获取thread失败:", error);
        // 创建空thread作为后备
        this.threads[linkId] = {
          id: link.thread_id,
          messages: [],
          members: [],
        };
      }
    },

    async sendMessage(link) {
      const linkId = this.getLinkId(link);
      if (!linkId) return;

      const messageText = this.messageTexts[linkId];
      if (!messageText || this.sendingMessage[linkId]) return;

      this.sendingMessage[linkId] = true;
      try {
        // 使用版本链接专用的thread API
        // 如果thread不存在，后端会自动创建
        // 注意：第一个参数是翻译版本ID（当前版本），第二个参数是目标版本ID
        const response = await useBaseFetch(
          `version/${this.version.id}/link/${link.joining_version_id}/thread`,
          {
            method: "POST",
            body: {
              body: messageText,
            },
          },
        );

        // 如果是第一次发送消息，保存thread_id
        if (response && response.thread_id && !link.thread_id) {
          link.thread_id = response.thread_id;
          // 初始化thread对象
          if (!this.threads[linkId]) {
            this.threads[linkId] = {
              id: response.thread_id,
              messages: [],
              members: [],
            };
          }
        }

        // 清空输入框
        this.messageTexts[linkId] = "";

        // 重新获取thread以显示新消息
        await this.fetchThread(link);

        this.$notify({
          group: "main",
          title: "成功",
          text: "消息已发送",
          type: "success",
        });
      } catch (error) {
        console.error("发送消息失败:", error);
        this.$notify({
          group: "main",
          title: "错误",
          text: "发送消息失败",
          type: "error",
        });
      } finally {
        this.sendingMessage[linkId] = false;
      }
    },

    getMessageAuthor(message, thread) {
      if (!message.author_id) return null;
      return thread.members?.find((m) => m.id === message.author_id);
    },

    isStaff(user) {
      return user?.role === "admin" || user?.role === "moderator";
    },

    renderMarkdown(text) {
      return renderString(text);
    },

    formatApprovalStatus(status) {
      const statusMap = {
        approved: "已批准",
        pending: "待审核",
        rejected: "已拒绝",
      };
      return statusMap[status] || status;
    },

    // 获取链接的唯一ID
    getLinkId(link) {
      if (!link) return null;
      // 优先使用joining_version_id，如果没有则使用一个稳定的索引或其他字段
      const id =
        link.joining_version_id ||
        link.id ||
        link.version_id ||
        link.target_version_id ||
        link.originalVersion?.id ||
        `link_${this.versionLinks.indexOf(link)}`;
      console.log("getLinkId返回:", id, "for link:", link); // 调试信息
      return id;
    },
  },
});
</script>

<style lang="scss" scoped>
.changelog-editor-spacing {
  padding-block: var(--gap-md);
}

.version-page {
  display: grid;

  grid-template:
    "title" auto
    "disk_url" auto
    "changelog" auto
    "translations" auto
    "dependencies" auto
    "metadata" auto
    "files" auto
    / 1fr;

  @media (min-width: 1200px) {
    grid-template:
      "title title" auto
      "disk_url metadata" auto
      "changelog metadata" auto
      "translations metadata" auto
      "dependencies metadata" auto
      "files metadata" auto
      "dummy metadata" 1fr
      / 1fr 20rem;
  }

  column-gap: var(--spacing-card-md);

  .version-page__title {
    grid-area: title;

    .version-header {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      margin-bottom: 1rem;
      gap: var(--spacing-card-md);

      h2,
      input[type="text"] {
        margin: 0;
        font-size: var(--font-size-2xl);
        font-weight: bold;
      }

      input[type="text"] {
        max-width: 100%;
        min-width: 0;
        flex-grow: 1;
        width: 2rem;
      }

      .featured {
        display: flex;
        align-items: center;
        gap: var(--spacing-card-xs);

        svg {
          height: 1.45rem;
        }
      }
    }

    .known-errors {
      margin-bottom: 1rem;
    }
  }

  h3 {
    font-size: var(--font-size-lg);
    margin: 0 0 0.5rem 0;
  }

  .version-page__changelog {
    grid-area: changelog;
    overflow-x: hidden;
  }

  .version-page__disk_url {
    grid-area: disk_url;
    overflow-x: hidden;
  }

  .version-page__translations {
    grid-area: translations;

    .translation-item {
      align-items: center;
      display: flex;
      gap: var(--spacing-card-sm);
      padding: var(--spacing-card-sm);

      .info {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-card-xs);

        .project-title {
          font-weight: bold;
        }

        .translation-details {
          color: var(--color-text-secondary);
          font-size: 0.875rem;
          display: flex;
          align-items: center;
          gap: 0.5rem;

          .separator {
            color: var(--color-text-disabled);
          }

          .version-info {
            font-weight: 500;
          }

          .date-info {
            font-size: 0.825rem;
          }
        }
      }
    }

    .no-translations {
      display: flex;
      align-items: center;
      gap: var(--spacing-card-sm);
      padding: var(--spacing-card-sm);
      color: var(--color-text-secondary);
    }

    .loading-indicator {
      padding: var(--spacing-card-sm);
      color: var(--color-text-secondary);
    }
  }

  .version-page__translations {
    grid-area: translations;

    .translation-item {
      display: flex;
      align-items: center;
      gap: var(--spacing-card-sm);
      padding: var(--spacing-card-xs) var(--spacing-card-sm);
      margin: 0 calc(var(--spacing-card-sm) * -1);
      border-radius: var(--size-rounded-sm);

      .info {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;

        .project-title {
          font-weight: 600;
          color: var(--color-text);
        }

        .translation-details {
          font-size: 0.875rem;
          color: var(--color-text-secondary);
        }
      }

      &:hover {
        background-color: var(--color-raised-bg);
      }
    }

    .no-translations {
      display: flex;
      align-items: center;
      gap: var(--spacing-card-sm);
      padding: var(--spacing-card-sm);
      color: var(--color-text-secondary);
    }

    .loading-indicator {
      padding: var(--spacing-card-sm);
      color: var(--color-text-secondary);
    }
  }

  .version-page__dependencies {
    grid-area: dependencies;

    .dependency {
      align-items: center;
      display: flex;
      gap: var(--spacing-card-sm);
      padding: var(--spacing-card-sm);

      .info {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-card-xs);

        .project-title {
          font-weight: bold;
        }

        .dep-type {
          color: var(--color-text-secondary);

          &.incompatible {
            color: var(--color-red);
          }

          &::first-letter {
            text-transform: capitalize;
          }
        }
      }

      button {
        margin-left: auto;
      }
    }

    .add-dependency {
      h4 {
        margin-bottom: var(--spacing-card-sm);
      }

      .input-group {
        &:not(:last-child) {
          margin-bottom: var(--spacing-card-sm);
        }

        .multiselect {
          width: 8rem;
          flex-grow: 1;
        }

        input {
          flex-grow: 2;
        }
      }
    }
  }

  .version-page__files {
    grid-area: files;

    .file {
      --text-color: var(--color-button-text);
      --background-color: var(--color-button-bg);

      &.primary {
        --background-color: var(--color-brand-highlight);
        --text-color: var(--color-button-text-active);
      }

      display: flex;
      align-items: center;

      font-weight: 500;
      color: var(--text-color);
      background-color: var(--background-color);
      padding: var(--spacing-card-sm) var(--spacing-card-bg);
      border-radius: var(--size-rounded-sm);

      svg {
        min-width: 1.1rem;
        min-height: 1.1rem;
        margin-right: 0.5rem;
      }

      .filename {
        word-wrap: anywhere;
      }

      .file-size {
        margin-left: 1ch;
        font-weight: 400;
        white-space: nowrap;
      }

      .file-type {
        margin-left: 1ch;
        font-style: italic;
        font-weight: 300;
      }

      .raised-multiselect {
        display: none;
        margin: 0 0.5rem;
        height: 40px;
        max-height: 40px;
        min-width: 235px;
      }

      .raised-button {
        margin-left: auto;
        background-color: var(--color-raised-bg);
      }

      &:not(:nth-child(2)) {
        margin-top: 0.5rem;
      }

      // TODO: Make file type editing  work on mobile
      @media (min-width: 600px) {
        .raised-multiselect {
          display: block;
        }
      }
    }

    .additional-files {
      h4 {
        margin-bottom: 0.5rem;
      }

      label {
        margin-top: 0.5rem;
      }
    }
  }
}

.version-page__metadata {
  grid-area: metadata;

  h4 {
    margin: 1rem 0 0.25rem 0;
  }

  .team-member {
    align-items: center;
    padding: 0.25rem 0.5rem;

    .member-info {
      overflow: hidden;
      margin: auto 0 auto 0.75rem;

      .name {
        font-weight: bold;
      }

      p {
        font-size: var(--font-size-sm);
        margin: 0.2rem 0;
      }
    }
  }
}

.separator {
  margin: var(--spacing-card-sm) 0;
}

.version-page__version-links {
  grid-area: dependencies;

  .version-link {
    align-items: center;
    display: flex;
    gap: var(--spacing-card-sm);
    padding: var(--spacing-card-sm);
    border-bottom: 1px solid var(--color-divider);

    &:last-child {
      border-bottom: none;
    }

    .info {
      display: flex;
      flex-direction: column;
      gap: var(--spacing-card-xs);
      flex-grow: 1;

      .project-title {
        font-weight: bold;
      }

      .version-info {
        color: var(--color-text-secondary);
        font-size: var(--font-size-sm);
      }

      .language-badge {
        display: inline-block;
        padding: 0.2rem 0.5rem;
        background: var(--color-raised-bg);
        border-radius: var(--radius-sm);
        font-size: var(--font-size-sm);
        margin-right: 0.5rem;
      }
    }

    button {
      margin-left: auto;
    }
  }

  .add-version-link {
    h4 {
      margin-bottom: var(--spacing-card-sm);
    }

    .approval-info-box {
      display: flex;
      gap: var(--spacing-card-sm);
      padding: var(--spacing-card-md);
      background: var(--color-bg-raised);
      border: 1px solid var(--color-divider);
      border-radius: var(--radius-md);
      margin-bottom: var(--spacing-card-md);

      svg {
        flex-shrink: 0;
        width: 20px;
        height: 20px;
        color: var(--color-brand);
        margin-top: 2px;
      }

      .approval-info-content {
        flex: 1;

        p {
          margin: 0 0 var(--spacing-card-xs) 0;
          color: var(--color-text);

          &:last-child {
            margin-bottom: 0;
          }

          strong {
            font-weight: 600;
          }
        }

        ul {
          margin: var(--spacing-card-xs) 0;
          padding-left: 1.5rem;
          list-style: disc;

          li {
            margin: var(--spacing-card-xs) 0;
            color: var(--color-text-secondary);

            strong {
              color: var(--color-text);
              font-weight: 600;
            }
          }
        }

        .highlight-green {
          color: var(--color-green);
          font-weight: 600;
          margin: 0 0.25rem;
        }

        .highlight-orange {
          color: var(--color-orange);
          font-weight: 600;
          margin: 0 0.25rem;
        }
      }
    }

    .input-group {
      &:not(:last-child) {
        margin-bottom: var(--spacing-card-sm);
      }

      .search-results {
        position: absolute;
        top: 100%;
        left: 0;
        right: 0;
        max-height: 200px;
        overflow-y: auto;
        background: var(--color-raised-bg);
        border: 1px solid var(--color-divider);
        border-radius: var(--radius-sm);
        margin-top: 0.25rem;
        z-index: 10;

        .search-result {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem;
          cursor: pointer;

          &:hover {
            background: var(--color-button-bg-hover);
          }
        }
      }

      .multiselect {
        width: 100%;
      }
    }

    .version-preview {
      margin-bottom: var(--spacing-card-sm);

      .selected-version-card {
        display: flex;
        align-items: center;
        gap: var(--spacing-card-sm);
        padding: var(--spacing-card-sm);
        background: var(--color-raised-bg);
        border: 1px solid var(--color-divider);
        border-radius: var(--radius-sm);

        .info {
          display: flex;
          flex-direction: column;
          gap: 0.25rem;

          .project-title {
            font-weight: 600;
          }

          .version-info {
            font-size: var(--font-size-sm);
            color: var(--color-text-secondary);
          }
        }
      }
    }

    .version-link-notice {
      margin-bottom: var(--spacing-card-sm);

      .notice-box {
        display: flex;
        align-items: center;
        gap: var(--spacing-card-sm);
        padding: var(--spacing-card-sm);
        background: var(--color-brand-bg);
        border: 1px solid var(--color-brand);
        border-radius: var(--radius-sm);
        color: var(--color-brand);

        svg {
          flex-shrink: 0;
          width: 1.25rem;
          height: 1.25rem;
        }
      }
    }
  }
}

.modal-package-mod {
  padding: var(--spacing-card-bg);
  display: flex;
  flex-direction: column;

  .markdown-body {
    margin-bottom: 1rem;
  }

  .multiselect {
    max-width: 20rem;
  }
}

/* 审核状态标签样式 */
.approval-status-tag {
  display: inline-block;
  padding: 0.2rem 0.5rem;
  margin-left: 0.5rem;
  border-radius: 0.25rem;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.025em;

  &.status-approved {
    background-color: var(--color-green-bg);
    color: var(--color-green);
    border: 1px solid var(--color-green);
  }

  &.status-pending {
    background-color: var(--color-orange-bg);
    color: var(--color-orange);
    border: 1px solid var(--color-orange);
  }

  &.status-rejected {
    background-color: var(--color-red-bg);
    color: var(--color-red);
    border: 1px solid var(--color-red);
  }
}

/* 在版本链接中的样式调整 */
.version-link {
  display: flex;
  align-items: center;

  .info {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 0.25rem;

    .project-title {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }
  }

  /* 审核状态标签放在信息右侧，垂直居中 */
  .approval-status-tag {
    margin-left: auto;
    margin-right: 0.5rem;
    flex-shrink: 0;
  }
}

// Thread相关样式
.message-toggle {
  margin-left: auto;
  padding: 0.375rem 0.75rem;
  font-size: 0.875rem;
  display: flex;
  align-items: center;
  gap: 0.25rem;

  svg {
    width: 1rem;
    height: 1rem;
  }

  &:hover {
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }
}

.thread-section {
  margin-top: 1rem;
  padding: 1rem;
  background: var(--color-bg);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-divider);
}

.thread-header {
  margin-bottom: 1rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--color-divider);

  h5 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-heading);
  }

  .thread-description {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
  }
}

.thread-messages {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.messages-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  max-height: 400px;
  overflow-y: auto;
  padding: 0.5rem;
}

.message-item {
  padding: 0.75rem;
  background: var(--color-raised-bg);
  border-radius: var(--radius-md);

  &.mod-message {
    background: var(--color-primary-bg);
    border: 1px solid var(--color-primary);
  }
}

.message-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.message-author {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-weight: 600;
  font-size: 0.9rem;
}

.message-time {
  font-size: 0.875rem;
  color: var(--color-text-disabled);
}

.message-body {
  position: relative;
}

.message-text {
  font-size: 0.95rem;
  line-height: 1.5;
  color: var(--color-text);

  :deep(p) {
    margin: 0.5rem 0;

    &:first-child {
      margin-top: 0;
    }

    &:last-child {
      margin-bottom: 0;
    }
  }
}

.status-change {
  padding: 0.5rem;
  background: var(--color-bg);
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  color: var(--color-text-secondary);
  text-align: center;
}

.no-messages {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  color: var(--color-text-secondary);

  svg {
    width: 2rem;
    height: 2rem;
    opacity: 0.5;
    margin-bottom: 0.5rem;
  }

  p {
    margin: 0;
    font-size: 0.9rem;
  }
}

.send-message {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid var(--color-divider);
}

.message-input {
  width: 100%;
  padding: 0.75rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  font-size: 0.95rem;
  resize: vertical;
  min-height: 60px;

  &:focus {
    outline: none;
    border-color: var(--color-primary);
  }
}

.message-actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.loading-thread {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  gap: 0.75rem;
  color: var(--color-text-secondary);

  svg {
    width: 1.5rem;
    height: 1.5rem;
  }
}

.btn-secondary {
  background: var(--color-bg);
  color: var(--color-text);
  border: 1px solid var(--color-divider);
  padding: 0.5rem 1rem;
  border-radius: var(--radius-md);
  cursor: pointer;

  &:hover:not(:disabled) {
    background: var(--color-raised-bg);
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.btn-small {
  padding: 0.375rem 0.75rem;
  font-size: 0.875rem;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.animate-spin {
  animation: spin 1s linear infinite;
}
/* 重新提交对话框样式 */
.resubmit-content {
  padding: 1rem;

  p {
    margin-bottom: 1rem;
    color: var(--color-text-secondary);
    font-size: 0.95rem;
  }
}

.resubmit-textarea {
  width: 100%;
  min-height: 120px;
  padding: 0.75rem;
  background: var(--color-raised-bg);
  border: 1px solid var(--color-divider);
  border-radius: var(--radius-md);
  font-size: 0.95rem;
  font-family: inherit;
  resize: vertical;

  &:focus {
    outline: none;
    border-color: var(--color-primary);
  }
}

.modal-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
  padding: 1rem;
  padding-top: 0;

  button {
    display: flex;
    align-items: center;
    gap: 0.25rem;

    svg {
      width: 1rem;
      height: 1rem;
    }
  }
}
</style>
