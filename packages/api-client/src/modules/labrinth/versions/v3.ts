import { AbstractModule } from '../../../core/abstract-module'
import type { UploadHandle, UploadProgress } from '../../../types/upload'
import type { Labrinth } from '../types'

export class LabrinthVersionsV3Module extends AbstractModule {
  public getModuleID(): string {
    return 'labrinth_versions_v3'
  }

  /**
   * Get versions for a project (v3)
   */
  public async getProjectVersions(
    id: string,
    options?: Labrinth.Versions.v3.GetProjectVersionsParams,
  ): Promise<Labrinth.Versions.v3.Version[]> {
    const params: Record<string, string> = {}
    if (options?.game_versions?.length) {
      params.game_versions = JSON.stringify(options.game_versions)
    }
    if (options?.loaders?.length) {
      params.loaders = JSON.stringify(options.loaders)
    }

    return this.client.request<Labrinth.Versions.v3.Version[]>(`/project/${id}/version`, {
      api: 'labrinth',
      version: 2, // TODO: move this to a versions v2 module to keep api-client clean and organized
      method: 'GET',
      params: Object.keys(params).length > 0 ? params : undefined,
    })
  }

  public async getVersion(id: string): Promise<Labrinth.Versions.v3.Version> {
    return this.client.request<Labrinth.Versions.v3.Version>(`/version/${id}`, {
      api: 'labrinth',
      version: 3,
      method: 'GET',
    })
  }

  public async getVersions(ids: string[]): Promise<Labrinth.Versions.v3.Version[]> {
    return this.client.request<Labrinth.Versions.v3.Version[]>(`/versions`, {
      api: 'labrinth',
      version: 3,
      method: 'GET',
      params: { ids: JSON.stringify(ids) },
    })
  }

  public async getVersionFromIdOrNumber(
    projectId: string,
    versionId: string,
  ): Promise<Labrinth.Versions.v3.Version> {
    return this.client.request<Labrinth.Versions.v3.Version>(
      `/project/${projectId}/version/${versionId}`,
      {
        api: 'labrinth',
        version: 3,
        method: 'GET',
      },
    )
  }

  /**
   * Create a new version for a project (v3)
   *
   * 返回 UploadHandle（与上游 API 形状一致）。
   * 当前底层走 fetch，progress 在 promise 解决时一次性触发 100%。
   * 未来可换 XMLHttpRequest 实现真实分片进度。
   *
   * BBSMC 集成：
   * - disk_only=true 时不上传文件，primary_file 设为 null
   * - 5 个网盘 input（quark_disk/xunlei_disk/baidu_disk/modrinth/curseforge）聚合为 disk_urls 数组
   * - language 类型强制 loaders=['language']，不发 game_versions
   * - software/language/curse 标识从 draftVersion.type 与 is_modpack 派生
   * - 兼容旧自动化脚本：旧脚本仍可发 disk_only/disk_urls/version_links 字段，行为不变
   */
  public createVersion(
    draftVersion: Labrinth.Versions.v3.DraftVersion,
    versionFiles: Labrinth.Versions.v3.DraftVersionFile[],
    projectType: Labrinth.Projects.v2.ProjectType | null = null,
  ): UploadHandle<Labrinth.Versions.v3.Version> {
    const formData = new FormData()

    const files = versionFiles.map((vf) => vf.file)
    const fileTypes = versionFiles.map((vf) => vf.fileType || null)

    const fileParts = files.map((file, i) => `${file.name}-${i === 0 ? 'primary' : i}`)

    const fileTypeMap = fileParts.reduce<Record<string, Labrinth.Versions.v3.FileType | null>>(
      (acc, key, i) => {
        acc[key] = fileTypes[i]
        return acc
      },
      {},
    )

    // BBSMC: 5 个网盘 input 聚合为 disk_urls 数组
    const aggregatedDisks = aggregateDiskUrls(draftVersion)
    const hasFiles = files.length > 0
    const isDiskOnly = !!draftVersion.disk_only

    // BBSMC: 把 BBSMC 自定义类型转换为 v3 标准 loader 格式
    // （v3 接口不接受 curse / software / language 字段，必须通过 loader 体系表达）
    const isLanguageType = draftVersion.type === 'language'
    const isSoftwareType = draftVersion.type === 'software'
    const isModpack = !!draftVersion.is_modpack
    const originalLoaders = draftVersion.loaders
    let loaders = originalLoaders
    let mrpackLoaders: string[] | undefined
    let softwareLoaders: string[] | undefined

    if (isLanguageType) {
      // language 类型：固定 loaders=['language']
      loaders = ['language']
    } else if (isModpack || projectType === 'modpack') {
      // 整合包：原 loaders 装入 mrpack_loaders 字段，loaders 设为 ['mrpack']
      mrpackLoaders = originalLoaders.filter((l) => l !== 'mrpack')
      loaders = ['mrpack']
    } else if (isSoftwareType) {
      // 软件资源：原 loaders 装入 software_loaders 字段，loaders 设为 ['software']
      softwareLoaders = originalLoaders.filter((l) => l !== 'software')
      loaders = ['software']
    }

    // 用 any 构造便于条件性 omit 字段（不发的字段从对象中完全移除而非传 [] / undefined，
    // 否则 JSON.stringify 会保留空数组，后端会把 key 当 loader_field 去查表，导致
    // "加载器字段 'xxx' 对于任何提供的加载器都不存在" 错误）
    const data: any = {
      project_id: draftVersion.project_id,
      version_number: draftVersion.version_number,
      name: draftVersion.name || draftVersion.version_number,
      changelog: draftVersion.changelog,
      dependencies: draftVersion.dependencies || [],
      version_type: draftVersion.version_type,
      featured: !!draftVersion.featured,
      file_parts: hasFiles ? fileParts : [],
      file_types: hasFiles ? fileTypeMap : {},
      // BBSMC: disk_only 模式下没有主文件
      primary_file: hasFiles ? fileParts[0] : isDiskOnly ? null : undefined,
      loaders,
      // BBSMC 自定义字段（仅 v3 InitialVersionData 显式接收的）
      disk_only: isDiskOnly,
      disk_urls: aggregatedDisks.length > 0 ? aggregatedDisks : null,
    }

    // BBSMC: 仅 minecraft 类型（含 modpack）发 game_versions
    if (!isLanguageType && !isSoftwareType) {
      data.game_versions = draftVersion.game_versions ?? []
    }

    // environment 仅在有值时发
    // BBSMC 已跟进上游 ef04dcc37：删除老 4 字段，environment 是单字段必填
    if (draftVersion.environment) {
      data.environment = draftVersion.environment
    }

    // version_links 仅 language 类型发
    if (isLanguageType && draftVersion.version_links && draftVersion.version_links.length > 0) {
      data.version_links = draftVersion.version_links
    }

    if (mrpackLoaders) {
      data.mrpack_loaders = mrpackLoaders
    }
    if (softwareLoaders) {
      data.software_loaders = softwareLoaders
    }

    formData.append('data', JSON.stringify(data))

    if (hasFiles) {
      files.forEach((file, i) => {
        formData.append(fileParts[i], new Blob([file]), file.name)
      })
    }

    return uploadFormDataWithProgress<Labrinth.Versions.v3.Version>(
      this.client,
      'POST',
      '/v3/version',
      formData,
    )
  }

  public async modifyVersion(
    versionId: string,
    data: Labrinth.Versions.v3.ModifyVersionRequest,
  ): Promise<Labrinth.Versions.v3.Version> {
    return this.client.request<Labrinth.Versions.v3.Version>(`/version/${versionId}`, {
      api: 'labrinth',
      version: 3,
      method: 'PATCH',
      body: data,
    })
  }

  public async deleteVersion(versionId: string): Promise<void> {
    return this.client.request(`/version/${versionId}`, {
      api: 'labrinth',
      version: 2,
      method: 'DELETE',
    })
  }

  /**
   * Add files to an existing version
   * 返回 UploadHandle（与 createVersion 一致）
   */
  public addFilesToVersion(
    versionId: string,
    versionFiles: Labrinth.Versions.v3.DraftVersionFile[],
  ): UploadHandle<Labrinth.Versions.v3.Version> {
    const formData = new FormData()

    const files = versionFiles.map((vf) => vf.file)
    const fileTypes = versionFiles.map((vf) => vf.fileType || null)

    const fileParts = files.map((file, i) => `${file.name}-${i}`)

    const fileTypeMap = fileParts.reduce<Record<string, Labrinth.Versions.v3.FileType | null>>(
      (acc, key, i) => {
        acc[key] = fileTypes[i]
        return acc
      },
      {},
    )

    formData.append('data', JSON.stringify({ file_types: fileTypeMap }))

    files.forEach((file, i) => {
      formData.append(fileParts[i], new Blob([file]), file.name)
    })

    return uploadFormDataWithProgress<Labrinth.Versions.v3.Version>(
      this.client,
      'POST',
      `/v2/version/${versionId}/file`,
      formData,
    )
  }
}

/**
 * 用 XMLHttpRequest 上传 FormData，提供真实分片进度（fetch 不支持上传进度）。
 *
 * - URL 由 client.config.labrinthBaseUrl + path 拼接（去掉重复斜杠）
 * - auth header 从 cookies (auth-token) 读取（与 BBSMC useBaseFetch 一致）
 * - Content-Type 由浏览器自动设置（multipart/form-data; boundary=...）
 * - 返回 UploadHandle 含 promise / onProgress / cancel
 */
function uploadFormDataWithProgress<T>(
  client: any,
  method: string,
  path: string,
  formData: FormData,
): UploadHandle<T> {
  const callbacks: Array<(p: UploadProgress) => void> = []
  const xhr = new XMLHttpRequest()

  // 拼接完整 URL
  const baseUrl: string = (client?.config?.labrinthBaseUrl ?? '').replace(/\/v\d+\/?$/, '').replace(/\/$/, '')
  const fullPath = path.startsWith('/') ? path : `/${path}`
  const url = `${baseUrl}${fullPath}`

  // 从 cookie 读 auth token（BBSMC 标准 auth 流程）
  let token: string | undefined
  if (typeof document !== 'undefined') {
    const match = document.cookie.split('; ').find((c) => c.startsWith('auth-token='))
    if (match) token = decodeURIComponent(match.split('=').slice(1).join('='))
  }

  const promise = new Promise<T>((resolve, reject) => {
    xhr.open(method, url, true)
    xhr.responseType = 'json'
    // 不开 withCredentials：auth 通过 Authorization header 传，与 BBSMC useBaseFetch 一致
    // 后端 CORS 用通配符 * 时也能正常工作

    if (token) {
      xhr.setRequestHeader('Authorization', token)
    }

    // 上传分片进度
    xhr.upload.addEventListener('progress', (e: ProgressEvent) => {
      if (!e.lengthComputable) return
      const progress: UploadProgress = {
        loaded: e.loaded,
        total: e.total,
        progress: e.total > 0 ? e.loaded / e.total : 0,
      }
      callbacks.forEach((cb) => {
        try {
          cb(progress)
        } catch (err) {
          console.error('UploadHandle progress callback error:', err)
        }
      })
    })

    xhr.addEventListener('load', () => {
      // 完成时确保 progress 跳到 100%（处理某些代理不发 final progress event 的情况）
      const finalProgress: UploadProgress = { loaded: 1, total: 1, progress: 1 }
      callbacks.forEach((cb) => {
        try {
          cb(finalProgress)
        } catch {
          // ignore
        }
      })

      if (xhr.status >= 200 && xhr.status < 300) {
        resolve(xhr.response as T)
      } else {
        // 模仿 ofetch 错误：把 response 挂在 err.data
        const data = xhr.response ?? {}
        const err: any = new Error(
          (data && (data.description || data.error)) || `HTTP ${xhr.status}`,
        )
        err.data = data
        err.statusCode = xhr.status
        reject(err)
      }
    })

    xhr.addEventListener('error', () => {
      const err: any = new Error('网络错误')
      err.data = { description: '网络请求失败，请稍后重试。' }
      reject(err)
    })

    xhr.addEventListener('abort', () => {
      const err: any = new Error('上传已取消')
      err.data = { description: '上传被用户取消。' }
      reject(err)
    })

    // Content-Type 不手动设，让浏览器自动设 multipart/form-data; boundary=xxx
    xhr.send(formData)
  })

  const handle: UploadHandle<T> = {
    promise,
    onProgress(callback) {
      callbacks.push(callback)
      return handle
    },
    cancel() {
      try {
        xhr.abort()
      } catch {
        // ignore
      }
    },
  }
  return handle
}

/**
 * BBSMC: 把 draftVersion 上的 5 个网盘 input 字段聚合为 disk_urls 数组
 * 顺序与旧 [version].vue 保持一致（quark/baidu/curseforge/modrinth/xunlei）
 */
function aggregateDiskUrls(
  draft: Labrinth.Versions.v3.DraftVersion,
): Labrinth.Versions.v3.QueryDisk[] {
  // 优先用 draft.disk_urls（如果调用者已聚合好），否则从 5 个 input 字段聚合
  if (draft.disk_urls && draft.disk_urls.length > 0) {
    return draft.disk_urls
  }

  const out: Labrinth.Versions.v3.QueryDisk[] = []
  const push = (platform: string, url?: string, display?: string) => {
    if (url && url.trim() !== '') out.push({ platform, url, display: display ?? 'default' })
  }
  push('quark', draft.quark_disk, draft.quark_disk_display)
  push('baidu', draft.baidu_disk, draft.baidu_disk_display)
  push('curseforge', draft.curseforge)
  push('modrinth', draft.modrinth)
  push('xunlei', draft.xunlei_disk, draft.xunlei_disk_display)
  return out
}

