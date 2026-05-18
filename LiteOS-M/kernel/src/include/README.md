这个文件夹中修改了两个地方，让rs文件能够生成。

`los_config.h`中把第41行的`#include "target_config.h"`删除掉，发现不影响编译。

`los_trace.h`中手动添加了第41 ~ 43行（内容如下），使得文件得以编译通过。
```c
#ifndef LOSCFG_TRACE_FRAME_MAX_PARAMS
#define LOSCFG_TRACE_FRAME_MAX_PARAMS 3
#endif
```