/* Test fixture only: never accepted by CakeRegistry. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/socket.h>
#include <sys/prctl.h>
#include <signal.h>
#include <sys/resource.h>
#include <sys/mman.h>
#include <arpa/inet.h>
int main(int argc, char **argv) {
  if (argc < 3) return 2;
  FILE *input = fopen(argv[argc-2], "r");
  FILE *proof = fopen(argv[argc-1], "r");
  if (!input || !proof) return 2;
  int mode = fgetc(input);
  if (mode == 'S') { raise(SIGSEGV); return 0; }
  if (mode == 'T') { prctl(PR_SET_NAME, "logos-cake-tmo", 0, 0, 0); if (fork() == 0) { sleep(30); return 0; } sleep(30); return 0; }
  if (mode == 'O') { for (int i=0;i<10000000;i++) putchar('x'); return 0; }
  struct rlimit memory, cpu;
  getrlimit(RLIMIT_AS, &memory); getrlimit(RLIMIT_CPU, &cpu);
  unsigned long long expected=0; unsigned port=0;
  if (fscanf(proof,"%llu %u",&expected,&port) != 2) return 2;
  if (memory.rlim_cur != expected || cpu.rlim_cur == RLIM_INFINITY) return 3;
  void *p=mmap(NULL,expected+4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
  if (p != MAP_FAILED) return 4;
  if (getenv("PATH") || access("/etc/passwd", F_OK)==0 || access("/bin/sh",F_OK)==0) return 5;
  if (open(argv[argc-2],O_WRONLY)>=0 || open("/escape",O_WRONLY|O_CREAT,0600)>=0) return 6;
  int sock=socket(AF_INET,SOCK_STREAM,0);
  struct sockaddr_in address={.sin_family=AF_INET,.sin_port=htons(port)};
  address.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
  if (sock>=0 && connect(sock,(struct sockaddr*)&address,sizeof(address))==0) return 7;
  puts("s VERIFIED UNSAT");
  return 0;
}
