package harness

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/pem"
	"math/big"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"testing"
	"time"
)

// covapp 和 service、baseline 放在同一个目录里：Main 结束时一起删掉
var covBuild struct {
	once sync.Once
	out  []byte
	err  error
}

// CovAppBinary e2e/covapp 编出来的可执行文件，见那里的说明
func CovAppBinary(t testing.TB) string {
	t.Helper()
	ServiceBinary(t) // 先让 build.dir 建出来
	covBuild.once.Do(func() {
		cmd := exec.Command("go", "build", "-o", filepath.Join(build.dir, "covapp"), "./covapp")
		cmd.Dir = ModuleDir()
		cmd.Env = append(os.Environ(), "GOWORK=off")
		covBuild.out, covBuild.err = cmd.CombinedOutput()
	})
	if covBuild.err != nil {
		t.Fatalf("build e2e/covapp: %v\n%s", covBuild.err, covBuild.out)
	}
	return filepath.Join(build.dir, "covapp")
}

// StartCovApp 起 covapp，参数原样交给它，不等就绪（它不监听）。Options 里只有 Env 对它有意义
func StartCovApp(t testing.TB, o Options, args ...string) *Process {
	t.Helper()
	o.NoWait = true
	return launch(t, "covapp", CovAppBinary(t), args, t.TempDir(), o)
}

// StartArgs 起 e2e 服务，命令行参数原样是 args：不像 Start 那样自己加 --config。
// 测 XONE_CONFIG、约定路径这些「没给 --config」时的找法用它。工作目录是 Process.Dir
func StartArgs(t testing.TB, o Options, args ...string) *Process {
	t.Helper()
	return launch(t, "service", ServiceBinary(t), args, t.TempDir(), o)
}

// WaitReadyWith 用 c 反复 GET url 直到 200，timeout 内没等到、或者进程先退出了就 t.Fatal。
// 给 HTTPS、h2c 这类 Start 自己的 http:// 就绪探测够不着的服务用（启动时传 Options.NoWait）
func (p *Process) WaitReadyWith(t testing.TB, c *http.Client, url string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for {
		resp, err := c.Get(url)
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == http.StatusOK {
				return
			}
		}
		select {
		case <-p.done:
			t.Fatalf("%s exited before it was ready: %v\n%s", p.name, p.exit, p.out.tail(80))
		case <-time.After(20 * time.Millisecond):
		}
		if time.Now().After(deadline) {
			t.Fatalf("%s not ready at %s within %v (last error: %v)\n%s", p.name, url, timeout, err, p.out.tail(80))
		}
	}
}

// SelfSignedCert 在 dir 里生成一张给 127.0.0.1 / localhost 的自签名证书，
// 返回证书、私钥文件的路径，和只信这张证书的 CertPool
func SelfSignedCert(t testing.TB, dir string) (certFile, keyFile string, pool *x509.CertPool) {
	t.Helper()
	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatalf("generate key: %v", err)
	}
	tmpl := &x509.Certificate{
		SerialNumber: big.NewInt(time.Now().UnixNano()),
		Subject:      pkix.Name{CommonName: "xone e2e"},
		NotBefore:    time.Now().Add(-time.Hour),
		NotAfter:     time.Now().Add(24 * time.Hour),
		KeyUsage:     x509.KeyUsageDigitalSignature,
		ExtKeyUsage:  []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth},
		IPAddresses:  []net.IP{net.ParseIP("127.0.0.1")},
		DNSNames:     []string{"localhost"},
	}
	der, err := x509.CreateCertificate(rand.Reader, tmpl, tmpl, &key.PublicKey, key)
	if err != nil {
		t.Fatalf("create certificate: %v", err)
	}
	keyDER, err := x509.MarshalECPrivateKey(key)
	if err != nil {
		t.Fatalf("marshal key: %v", err)
	}
	certFile, keyFile = filepath.Join(dir, "cert.pem"), filepath.Join(dir, "key.pem")
	writeFile(t, certFile, pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: der}))
	writeFile(t, keyFile, pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: keyDER}))
	cert, err := x509.ParseCertificate(der)
	if err != nil {
		t.Fatalf("parse certificate: %v", err)
	}
	pool = x509.NewCertPool()
	pool.AddCert(cert)
	return certFile, keyFile, pool
}
