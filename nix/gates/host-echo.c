/*
 * host-echo — the NETIP-08 soak gate's host-side echo peer (16-05,
 * D-12): the in-guest tcpecho's echo semantics re-keyed as a HOST
 * POSIX C program (host cc, not the Plan 9 toolchain — this is host
 * tooling like the nix gate scripts) and made CONCURRENT.  A poll()
 * loop services the listener and every accepted connection together;
 * the sequential one-conversation-at-a-time accept loop of the
 * in-guest tcpecho (and of the r9 xtask HostEcho precedent) is
 * deliberately NOT copied — 8 simultaneously-open tcpblast
 * connections are the soak's burst leg, and a sequential peer would
 * serialize them into a queue (research Pitfall 7), blurring exactly
 * the wedge semantics the gate exists to expose.
 *
 * argv: host-echo <bindaddr> <port>   (the soak gate passes the
 * netaddr seam's tap_gwv4 and blastport — the peer binds the BRIDGE
 * address only, never the wildcard, so it is reachable solely from
 * the bridge's subnet and the host itself; T-16-11).
 *
 * Markers on stdout (the gate-owned vocabulary — D-12's no-tool-drift
 * rule; each flushed so a redirected log carries it promptly):
 *   hostecho: listening <addr>:<port>
 *   hostecho: conn <n>
 *   hostecho: closed <n> <bytes-in> <bytes-out>
 *
 * Lifecycle: the peer runs as the soak app's own child and dies with
 * it — SIGTERM/SIGINT set a stop flag observed by the poll cadence
 * (the gate's EXIT trap sends it; no daemon, no systemd).  Bind and
 * listen failures are loud: perror + non-zero exit, the netblast
 * error posture adapted to C.  SIGPIPE is ignored so a client closing
 * mid-write costs that one connection (EPIPE handled per-fd), never
 * the server.
 */

#include <errno.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

/* fds[0] is the listener; fds[1..nconn] are accepted connections,
 * each paired with conns[i] for the marker accounting.  64 slots
 * dwarf the gate's 8-way burst; a fuller table just leaves the
 * backlog queued (never refused blind) until a slot frees. */
#define MAXCONN	64
#define BUFSZ	65536

struct conn {
	int fd;
	long num;
	long bytes_in;
	long bytes_out;
};

static struct pollfd fds[MAXCONN+1];
static struct conn conns[MAXCONN+1];
static int nconn;

static volatile sig_atomic_t stopping = 0;

static void
stopsig(int sig)
{
	(void)sig;
	stopping = 1;
}

/* write n bytes fully, partial-write-safe (the tcpecho semantics: the
 * byte-compare belongs to the guest; the peer's job is to give every
 * byte back).  -1 on a failed write (the caller closes the fd). */
static long
writeall(int fd, char *buf, long n)
{
	long off, w;

	off = 0;
	while(off < n){
		w = write(fd, buf+off, n-off);
		if(w < 0){
			if(errno == EINTR)
				continue;
			return -1;
		}
		off += w;
	}
	return off;
}

/* close one connection slot: the marker line (the conn NUMBER is the
 * identity — fd indices are not), then compact by moving the last
 * slot into the freed one. */
static void
closeconn(int i)
{
	printf("hostecho: closed %ld %ld %ld\n",
		conns[i].num, conns[i].bytes_in, conns[i].bytes_out);
	fflush(stdout);
	close(conns[i].fd);
	fds[i] = fds[nconn];
	conns[i] = conns[nconn];
	nconn--;
}

int
main(int argc, char *argv[])
{
	struct sockaddr_in sa;
	struct sigaction act;
	char buf[BUFSZ];
	struct conn *c;
	int lfd, i, n, one;
	long connnum;
	struct in_addr bindaddr;
	unsigned short port;
	char *end;

	if(argc != 3){
		fprintf(stderr, "usage: host-echo <bindaddr> <port>\n");
		exit(1);
	}
	port = (unsigned short)strtoul(argv[2], &end, 10);
	if(*argv[2] == '\0' || *end != '\0' || port == 0){
		fprintf(stderr, "host-echo: bad port '%s' (want 1-65535)\n", argv[2]);
		exit(1);
	}
	if(inet_pton(AF_INET, argv[1], &bindaddr) != 1){
		fprintf(stderr, "host-echo: bad bind address '%s' (want a literal IPv4 dotted quad — the gate passes the seam's tap_gwv4)\n", argv[1]);
		exit(1);
	}

	memset(&act, 0, sizeof act);
	act.sa_handler = stopsig;
	sigaction(SIGTERM, &act, NULL);
	sigaction(SIGINT, &act, NULL);
	signal(SIGPIPE, SIG_IGN);

	lfd = socket(AF_INET, SOCK_STREAM, 0);
	if(lfd < 0){
		perror("host-echo: socket");
		exit(1);
	}
	one = 1;
	setsockopt(lfd, SOL_SOCKET, SO_REUSEADDR, &one, sizeof one);
	memset(&sa, 0, sizeof sa);
	sa.sin_family = AF_INET;
	sa.sin_addr = bindaddr;
	sa.sin_port = htons(port);
	if(bind(lfd, (struct sockaddr*)&sa, sizeof sa) < 0){
		perror("host-echo: bind");
		exit(1);
	}
	if(listen(lfd, 16) < 0){
		perror("host-echo: listen");
		exit(1);
	}
	printf("hostecho: listening %s:%s\n", argv[1], argv[2]);
	fflush(stdout);

	fds[0].fd = lfd;
	fds[0].events = POLLIN;
	nconn = 0;
	connnum = 0;

	while(!stopping){
		n = poll(fds, nconn+1, 200);
		if(n < 0){
			if(errno == EINTR)
				continue;
			perror("host-echo: poll");
			exit(1);
		}
		if(n == 0)
			continue;
		if((fds[0].revents & POLLIN) && nconn < MAXCONN){
			int dfd;

			dfd = accept(lfd, NULL, NULL);
			if(dfd < 0){
				if(errno != EINTR && errno != ECONNABORTED){
					perror("host-echo: accept");
					exit(1);
				}
			} else {
				connnum++;
				nconn++;
				fds[nconn].fd = dfd;
				fds[nconn].events = POLLIN;
				conns[nconn].fd = dfd;
				conns[nconn].num = connnum;
				conns[nconn].bytes_in = 0;
				conns[nconn].bytes_out = 0;
				printf("hostecho: conn %ld\n", connnum);
				fflush(stdout);
			}
		}
		if(fds[0].revents & (POLLERR|POLLHUP|POLLNVAL)){
			fprintf(stderr, "host-echo: listener poll error (revents 0x%x)\n", fds[0].revents);
			exit(1);
		}
		/* service readable connections; iterate DOWNWARD because
		 * closeconn compacts by moving the last slot into the
		 * closed one's place */
		for(i = nconn; i >= 1; i--){
			if(!(fds[i].revents & (POLLIN|POLLERR|POLLHUP|POLLNVAL)))
				continue;
			c = &conns[i];
			n = read(c->fd, buf, sizeof buf);
			if(n <= 0){
				/* EOF (0) or error: the conversation is
				 * over either way — close, mark, move on */
				closeconn(i);
				continue;
			}
			c->bytes_in += n;
			if(writeall(c->fd, buf, n) < 0){
				closeconn(i);
				continue;
			}
			c->bytes_out += n;
		}
	}
	/* the stop flag: close whatever remains (clean teardown when the
	 * gate's trap sends SIGTERM), then exit 0 */
	for(i = nconn; i >= 1; i--)
		closeconn(i);
	close(lfd);
	exit(0);
}
