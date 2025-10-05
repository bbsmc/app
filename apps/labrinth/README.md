
# BBSMC 后端指南

## 初始化
#### 首先安装  sqlx CLI，

```bash
cargo install --git https://github.com/launchbadge/sqlx sqlx-cli --no-default-features --features postgres,rustls
```

#### 初始化数据库
```bash
sqlx database setup
```


#### 提交代码
**提交前，请确保已进行下方操作**
- cargo fmt  已运行。
- cargo clippy  已运行。
- cargo check  已运行。
- cargo sqlx prepare  已运行

注意：如果在运行后遇到sqlx“未找到查询”的问题cargo sqlx prepare


### Loader讲解

    什么是loader? 在Modrinth的设计中, loader起到关键作用，接下来我们将讲解每个loader表的设计和关联
    
    loader:  加载器表, 如spigot, paper, forge等加载核心, mrpack, software这些更高一级的加载器都要在这里进行注册
    project_types:  资源类型表, 如 mod mrpack plugin software 这一类资源类型, 注册新的资源板块需要在这里注册后端的资源类型
    loader_peoject_types:  资源关联加载器表 joining_loader_id 是loader的加载器id, joining_project_type_id 是project_types的id

    目前来讲，如果是非mrpack  sofrware 这类更高一层的加载器，这种不需要内嵌其他loader的loader，则只需要上面两个步骤即可，无需做其他的添加

    现在我们将开始注册mrpack这类加载器(loaders)
    首先让loaders可以选择更多的loader
    loader_fields:  加载器字段, 可以是   array_enum  去兼容多种其他loader例如sofrware加载器可以选择linux windows macos 这些loader
                    需要注册其对应的enum_type, 若 field_type 是 array_enum 的情况下， 对应的enum在 loader_field_enums 里注册
    loader_fields_loaders:   让 loader  成为 loaders, 就是让loader绑定可以可选字段
    loader_field_enums：  加载器字段，例如 mrpack_loaders  sortware_loaders  注册这个枚举在这里， 注册完成以后需在 loader_field_enum_values  添加对应的字段
                          例如 game_versions（游戏版本） 这个，枚举的是所有上游戏版本，该版本的字段会定时去mojang拉取并更新到数据库内
                              mrpack_loaders  这个整合包枚举里就有forge fabric neoforge 等一些整合包加载器
    
    loader_field_enum_values:  加载器字段枚举列表，每个字段里有哪些枚举在这里注册，如forge fabric  linux macos windows 1.12.2 1.21.1 这一类可选 多选的枚举，只有在这里注册以后才可以被选择

    