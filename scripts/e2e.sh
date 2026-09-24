#!/bin/sh
# 跑 e2e/ 下的真实 Web 服务测试：真的 PostgreSQL、真的 MySQL、真的 Redis、真的进程和信号。
#
#   scripts/e2e.sh                 # 全部（压测除外）
#   scripts/e2e.sh -run Smoke      # 后面的参数原样交给 go test
#   scripts/e2e.sh --load -run Load  # 连压测一起跑（XONE_E2E_LOAD=1），压测一轮十来分钟
#
# 压测要独占机器：压测器、被测服务、PG 本来就挤在同一台机器上，再有别的负载数字就没法看了。
#
# PG / MySQL / Redis 已经在跑就直接用，没在跑就按本机的装法拉起来。
# 连接参数都能用环境变量盖掉，默认值和 e2e/harness 里的一致。
#
# 不在 CI 里、也不在 scripts/test.sh 里：那两处没有数据库，e2e 的每个测试
# 在 XONE_E2E 不为 1 时都会跳过。
set -e
cd "$(dirname "$0")/.."

# --load 可以写在任意位置，其余参数原样留给 go test
timeout=30m
for a; do
  shift
  if [ "$a" = --load ]; then
    export XONE_E2E_LOAD=1
    timeout=60m
    continue
  fi
  set -- "$@" "$a"
done

: "${XONE_E2E_PG_ADDR:=127.0.0.1:5432}"
: "${XONE_E2E_PG_USER:=xone}"
: "${XONE_E2E_PG_PASSWORD:=e2e-secret-pw}"
: "${XONE_E2E_PG_DB:=xone_e2e}"
: "${XONE_E2E_REDIS_ADDR:=127.0.0.1:6379}"
: "${XONE_E2E_MYSQL_ADDR:=127.0.0.1:3306}"
: "${XONE_E2E_MYSQL_USER:=xone}"
: "${XONE_E2E_MYSQL_PASSWORD:=e2e-secret-pw}"
: "${XONE_E2E_MYSQL_DB:=xone_e2e}"
# pg_ctlcluster 的版本和集群名，故意不加引号：它是两个参数
: "${XONE_E2E_PG_CLUSTER:=16 main}"
export XONE_E2E_PG_ADDR XONE_E2E_PG_USER XONE_E2E_PG_PASSWORD XONE_E2E_PG_DB XONE_E2E_REDIS_ADDR
export XONE_E2E_MYSQL_ADDR XONE_E2E_MYSQL_USER XONE_E2E_MYSQL_PASSWORD XONE_E2E_MYSQL_DB

pg_host=${XONE_E2E_PG_ADDR%:*}
pg_port=${XONE_E2E_PG_ADDR##*:}
mysql_host=${XONE_E2E_MYSQL_ADDR%:*}
mysql_port=${XONE_E2E_MYSQL_ADDR##*:}
redis_host=${XONE_E2E_REDIS_ADDR%:*}
redis_port=${XONE_E2E_REDIS_ADDR##*:}

# wait_for 名字 命令...：命令成功为止，最多等 10 秒
wait_for() {
  what=$1
  shift
  i=0
  until "$@" >/dev/null 2>&1; do
    i=$((i + 1))
    [ "$i" -lt 50 ] || { echo "✗ $what 没起来"; exit 1; }
    sleep 0.2
  done
}

# ---- PostgreSQL ----
if pg_isready -q -h "$pg_host" -p "$pg_port"; then
  echo "✓ PostgreSQL 已在 $XONE_E2E_PG_ADDR 运行"
else
  echo "== 启动 PostgreSQL（pg_ctlcluster $XONE_E2E_PG_CLUSTER start）"
  # shellcheck disable=SC2086
  pg_ctlcluster $XONE_E2E_PG_CLUSTER start
  wait_for PostgreSQL pg_isready -q -h "$pg_host" -p "$pg_port"
  echo "✓ PostgreSQL 已启动"
fi
# 用测试的账号连一次：账号或库不对的话在这里说一遍，而不是每个用例各报一遍
if ! PGPASSWORD=$XONE_E2E_PG_PASSWORD psql -h "$pg_host" -p "$pg_port" -U "$XONE_E2E_PG_USER" \
  -d "$XONE_E2E_PG_DB" -tAc 'SELECT 1' >/dev/null 2>&1; then
  cat <<TIP
✗ 用 $XONE_E2E_PG_USER 连不上 $XONE_E2E_PG_ADDR 上的库 $XONE_E2E_PG_DB。第一次跑的话先建账号和库：

  su postgres -c "psql -c \"CREATE ROLE $XONE_E2E_PG_USER LOGIN PASSWORD '<密码>'\""
  su postgres -c "createdb -O $XONE_E2E_PG_USER $XONE_E2E_PG_DB"

密码用 XONE_E2E_PG_PASSWORD 告诉本脚本。
TIP
  exit 1
fi

# ---- MySQL ----
# 密码经 MYSQL_PWD 交给客户端，不出现在命令行上（ps 看得见命令行）
mysql_ping() { MYSQL_PWD=$XONE_E2E_MYSQL_PASSWORD mysqladmin -h "$mysql_host" -P "$mysql_port" -u "$XONE_E2E_MYSQL_USER" ping; }
if mysql_ping >/dev/null 2>&1; then
  echo "✓ MySQL 已在 $XONE_E2E_MYSQL_ADDR 运行"
else
  echo "== 启动 MySQL（service mysql start）"
  service mysql start >/dev/null
  wait_for MySQL mysql_ping
  echo "✓ MySQL 已启动"
fi
if ! MYSQL_PWD=$XONE_E2E_MYSQL_PASSWORD mysql -h "$mysql_host" -P "$mysql_port" -u "$XONE_E2E_MYSQL_USER" \
  -D "$XONE_E2E_MYSQL_DB" -Nse 'SELECT 1' >/dev/null 2>&1; then
  cat <<TIP
✗ 用 $XONE_E2E_MYSQL_USER 连不上 $XONE_E2E_MYSQL_ADDR 上的库 $XONE_E2E_MYSQL_DB。第一次跑的话先建账号和库：

  mysql -uroot -e "CREATE DATABASE $XONE_E2E_MYSQL_DB; CREATE USER '$XONE_E2E_MYSQL_USER'@'127.0.0.1' IDENTIFIED BY '<密码>';
    GRANT ALL ON $XONE_E2E_MYSQL_DB.* TO '$XONE_E2E_MYSQL_USER'@'127.0.0.1'"

密码用 XONE_E2E_MYSQL_PASSWORD 告诉本脚本。
TIP
  exit 1
fi

# ---- Redis ----
if redis-cli -h "$redis_host" -p "$redis_port" ping >/dev/null 2>&1; then
  echo "✓ Redis 已在 $XONE_E2E_REDIS_ADDR 运行"
else
  echo "== 启动 Redis（redis-server --port $redis_port --daemonize yes --save '' --appendonly no）"
  # 不落盘：脚本此刻的工作目录是仓库根，Redis 的 dir 默认就是它。带着默认的
  # save 规则（300 秒内 100 次改动就 BGSAVE），跑一轮 e2e 就会在仓库根留下 dump.rdb
  redis-server --port "$redis_port" --daemonize yes --save '' --appendonly no >/dev/null
  wait_for Redis redis-cli -h "$redis_host" -p "$redis_port" ping
  echo "✓ Redis 已启动"
fi

# 和 scripts/test.sh 一样 GOWORK=off：测的是 e2e/go.mod 自己解出来的依赖
cd e2e
export XONE_E2E=1 GOWORK=off
exec go test -count=1 -v -timeout "$timeout" ./... "$@"
