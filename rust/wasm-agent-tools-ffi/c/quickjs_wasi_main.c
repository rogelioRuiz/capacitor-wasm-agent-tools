#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "quickjs.h"

static JSValue console_write_impl(JSContext *ctx, int argc, JSValueConst *argv, FILE *stream) {
    int i;

    for (i = 0; i < argc; i++) {
        const char *text = JS_ToCString(ctx, argv[i]);
        if (!text) {
            return JS_EXCEPTION;
        }
        if (i > 0) {
            fputc(' ', stream);
        }
        fputs(text, stream);
        JS_FreeCString(ctx, text);
    }
    fputc('\n', stream);
    fflush(stream);
    return JS_UNDEFINED;
}

static JSValue js_console_log(JSContext *ctx, JSValueConst this_val,
                              int argc, JSValueConst *argv) {
    (void)this_val;
    return console_write_impl(ctx, argc, argv, stdout);
}

static JSValue js_console_error(JSContext *ctx, JSValueConst this_val,
                                int argc, JSValueConst *argv) {
    (void)this_val;
    return console_write_impl(ctx, argc, argv, stderr);
}

static int install_console(JSContext *ctx) {
    JSValue global_obj = JS_GetGlobalObject(ctx);
    JSValue console = JS_GetPropertyStr(ctx, global_obj, "console");

    if (!JS_IsObject(console)) {
        JS_FreeValue(ctx, console);
        console = JS_NewObject(ctx);
        if (JS_IsException(console)) {
            JS_FreeValue(ctx, global_obj);
            return -1;
        }
        if (JS_SetPropertyStr(ctx, global_obj, "console", JS_DupValue(ctx, console)) < 0) {
            JS_FreeValue(ctx, console);
            JS_FreeValue(ctx, global_obj);
            return -1;
        }
    }

    if (JS_SetPropertyStr(ctx, console, "log",
                          JS_NewCFunction(ctx, js_console_log, "log", 1)) < 0) {
        JS_FreeValue(ctx, console);
        JS_FreeValue(ctx, global_obj);
        return -1;
    }
    if (JS_SetPropertyStr(ctx, console, "warn",
                          JS_NewCFunction(ctx, js_console_error, "warn", 1)) < 0) {
        JS_FreeValue(ctx, console);
        JS_FreeValue(ctx, global_obj);
        return -1;
    }
    if (JS_SetPropertyStr(ctx, console, "error",
                          JS_NewCFunction(ctx, js_console_error, "error", 1)) < 0) {
        JS_FreeValue(ctx, console);
        JS_FreeValue(ctx, global_obj);
        return -1;
    }

    JS_FreeValue(ctx, console);
    JS_FreeValue(ctx, global_obj);
    return 0;
}

static void dump_exception(JSContext *ctx) {
    JSValue exception = JS_GetException(ctx);
    JSValue stack = JS_GetPropertyStr(ctx, exception, "stack");
    const char *message = JS_ToCString(ctx, exception);
    const char *stack_text = NULL;

    if (JS_IsString(stack)) {
        stack_text = JS_ToCString(ctx, stack);
    }

    if (message) {
        fputs(message, stderr);
        fputc('\n', stderr);
    }

    if (stack_text && (!message || strcmp(stack_text, message) != 0)) {
        fputs(stack_text, stderr);
        fputc('\n', stderr);
    } else if (!message) {
        fputs("quickjs: unprintable exception\n", stderr);
    }
    fflush(stderr);

    if (stack_text) {
        JS_FreeCString(ctx, stack_text);
    }
    if (message) {
        JS_FreeCString(ctx, message);
    }
    JS_FreeValue(ctx, stack);
    JS_FreeValue(ctx, exception);
}

static int read_stdin(char **out_buf, size_t *out_len) {
    size_t capacity = 4096;
    size_t length = 0;
    char *buf = malloc(capacity);

    if (!buf) {
        return -1;
    }

    for (;;) {
        size_t remaining = capacity - length;
        size_t nread;

        if (remaining == 0) {
            size_t next_capacity = capacity * 2;
            char *next_buf = realloc(buf, next_capacity);
            if (!next_buf) {
                free(buf);
                return -1;
            }
            buf = next_buf;
            capacity = next_capacity;
            remaining = capacity - length;
        }

        nread = fread(buf + length, 1, remaining, stdin);
        length += nread;

        if (nread < remaining) {
            if (ferror(stdin)) {
                free(buf);
                return -1;
            }
            break;
        }
    }

    *out_buf = buf;
    *out_len = length;
    return 0;
}

static int write_result(JSContext *ctx, JSValue result) {
    const char *text;

    if (JS_IsUndefined(result)) {
        return 0;
    }

    text = JS_ToCString(ctx, result);
    if (!text) {
        return -1;
    }

    fputs(text, stdout);
    fputc('\n', stdout);
    fflush(stdout);
    JS_FreeCString(ctx, text);
    return 0;
}

int main(int argc, char **argv) {
    JSRuntime *rt;
    JSContext *ctx;
    JSValue result;
    char *source = NULL;
    const char *inline_code = NULL;
    size_t source_len = 0;
    int i;
    int exit_code = 0;

    for (i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "-e")) {
            if (i + 1 >= argc) {
                fputs("quickjs: -e requires an argument\n", stderr);
                return 2;
            }
            inline_code = argv[++i];
        } else {
            fprintf(stderr, "quickjs: unsupported argument: %s\n", argv[i]);
            return 2;
        }
    }

    rt = JS_NewRuntime();
    if (!rt) {
        fputs("quickjs: failed to create runtime\n", stderr);
        return 1;
    }

    ctx = JS_NewContext(rt);
    if (!ctx) {
        JS_FreeRuntime(rt);
        fputs("quickjs: failed to create context\n", stderr);
        return 1;
    }

    if (install_console(ctx) < 0) {
        dump_exception(ctx);
        JS_FreeContext(ctx);
        JS_FreeRuntime(rt);
        return 1;
    }

    if (inline_code) {
        source = (char *)inline_code;
        source_len = strlen(inline_code);
    } else if (read_stdin(&source, &source_len) < 0) {
        fprintf(stderr, "quickjs: failed to read stdin: %s\n", strerror(errno));
        JS_FreeContext(ctx);
        JS_FreeRuntime(rt);
        return 1;
    }

    result = JS_Eval(ctx, source, source_len, "<stdin>", JS_EVAL_TYPE_GLOBAL);
    if (JS_IsException(result)) {
        dump_exception(ctx);
        exit_code = 1;
    } else if (write_result(ctx, result) < 0) {
        dump_exception(ctx);
        exit_code = 1;
    }

    JS_FreeValue(ctx, result);
    if (!inline_code) {
        free(source);
    }
    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);
    return exit_code;
}
