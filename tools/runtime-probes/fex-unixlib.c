/* Owned Darwin companion ABI probe. Does not load Wine or start Steam. */
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

typedef uint32_t (*unix_call)(void *);
struct advice { void *address; size_t size; int32_t advice; uint32_t pad; };
struct mapping { int32_t fd; uint64_t size; void *result; };
struct stats_mapping { void *base; uint32_t size; uint32_t maximum; };
#define CHECK(test) do { if (!(test)) { fprintf(stderr, "FEX_UNIXLIB_FAIL line=%d\n", __LINE__); return 1; } } while (0)

int main(int argc, char **argv)
{
    CHECK(argc == 2);
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    CHECK(library != NULL);
    unix_call *calls = (unix_call *)dlsym(library, "__wine_unix_call_funcs");
    CHECK(calls != NULL);
    uint32_t pid = 0;
    CHECK(calls[7](&pid) == 0 && pid == (uint32_t)getpid());
    bool enabled = true;
    uint64_t flags = 1;
    struct advice advice = {0};
    advice.advice = 14;
    CHECK(calls[0](&enabled) == 0xc00000bb);
    CHECK(calls[1](&flags) == 0xc00000bb);
    CHECK(calls[2](&advice) == 0xc00000bb);
    FILE *file = tmpfile();
    CHECK(file != NULL);
    CHECK(fwrite("FEX ABI", 1, 7, file) == 7 && fflush(file) == 0);
    struct mapping mapping = {0};
    mapping.fd = dup(fileno(file));
    mapping.size = 7;
    CHECK(mapping.fd >= 0 && calls[6](&mapping) == 0);
    CHECK(mapping.result != NULL && memcmp(mapping.result, "FEX ABI", 7) == 0);
    errno = 0;
    CHECK(fcntl(mapping.fd, F_GETFD) == -1 && errno == EBADF);
    CHECK(munmap(mapping.result, 7) == 0);
    CHECK(fclose(file) == 0);

    /* Exercise the actual stats handlers under a permissive umask: inherited
     * process policy must not hide group/other access requested by the runtime. */
    char name[64];
    snprintf(name, sizeof(name), "/fex-%ld-stats", (long)getpid());
    errno = 0;
    int stats_fd = shm_open(name, O_RDWR, 0);
    if (stats_fd >= 0) close(stats_fd);
    CHECK(stats_fd == -1 && errno == ENOENT);
    uint32_t page = (uint32_t)getpagesize();
    struct stats_mapping stats = {NULL, page, page * 4};
    mode_t previous_umask = umask(0);
    uint32_t status = calls[4](&stats);
    umask(previous_umask);
    CHECK(status == 0 && stats.base != NULL);
    stats_fd = shm_open(name, O_RDWR, 0);
    struct stat metadata = {0};
    int secure = stats_fd >= 0 && fstat(stats_fd, &metadata) == 0
        && (metadata.st_mode & 0777) == (S_IRUSR | S_IWUSR) && metadata.st_uid == geteuid();
    int shared = 0;
    if (secure) {
        unsigned char *peer = mmap(NULL, page, PROT_READ | PROT_WRITE, MAP_SHARED, stats_fd, 0);
        if (peer != MAP_FAILED) {
            ((unsigned char *)stats.base)[0] = 0x5a;
            peer[1] = 0x3c;
            shared = peer[0] == 0x5a && ((unsigned char *)stats.base)[1] == 0x3c;
            munmap(peer, page);
        }
    }
    /* Clean up even if the permission regression is detected. */
    if (stats_fd >= 0) close(stats_fd);
    CHECK(munmap(stats.base, stats.maximum) == 0);
    CHECK(calls[5](NULL) == 0);
    errno = 0;
    stats_fd = shm_open(name, O_RDWR, 0);
    if (stats_fd >= 0) close(stats_fd);
    CHECK(stats_fd == -1 && errno == ENOENT);
    if (!secure || !shared)
        fprintf(stderr, "FEX_STATS_FAIL mode=%03o uid=%lu secure=%d shared=%d\n",
            metadata.st_mode & 0777, (unsigned long)metadata.st_uid, secure, shared);
    CHECK(secure && shared);
    puts("NUCLEON_FEX_STATS_PRIVATE_OK mode=0600 shared-read-write-unlink");
    CHECK(dlclose(library) == 0);
    puts("NUCLEON_FEX_UNIXLIB_OK");
    return 0;
}
