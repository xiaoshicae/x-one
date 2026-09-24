package main

import (
	"errors"
	"log/slog"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/xiaoshicae/x-one/e2e/service/store"
	"github.com/xiaoshicae/x-one/xmetric"
)

// MySQL 上的用户接口：和 PG 那一组同样的表结构，走第二个 xgorm 实例 xgorm.C("mysql")，
// 不经缓存——测的是「多实例各管各的」，不是三级读。
//
//	POST   /mysql/users        201 返回新用户
//	GET    /mysql/users/:id    200 / 404
//	PUT    /mysql/users/:id    200 / 404
//	DELETE /mysql/users/:id    204 / 404
//	GET    /mysql/sleep?ms=N   服务端 SELECT SLEEP(N/1000)，用请求的 ctx：测优雅退出时在途的查询做不做得完
func mysqlRoutes(e *gin.Engine) {
	e.POST("/mysql/users", createMySQLUser)
	e.GET("/mysql/users/:id", getMySQLUser)
	e.PUT("/mysql/users/:id", updateMySQLUser)
	e.DELETE("/mysql/users/:id", deleteMySQLUser)
	e.GET("/mysql/sleep", sleepMySQL)
}

func createMySQLUser(c *gin.Context) {
	ctx := c.Request.Context()
	var req userReq
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	u := store.User{Name: req.Name, Email: req.Email}
	if err := store.CreateMySQLUser(ctx, &u); err != nil {
		slog.ErrorContext(ctx, "create mysql user failed", "error", err)
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	xmetric.CounterInc("mysql_users_created_total")
	c.JSON(http.StatusCreated, u)
}

func getMySQLUser(c *gin.Context) {
	ctx := c.Request.Context()
	id, ok := userID(c)
	if !ok {
		return
	}
	u, err := store.GetMySQLUser(ctx, id)
	if mysqlFailed(c, "read mysql user failed", id, err) {
		return
	}
	c.JSON(http.StatusOK, u)
}

func updateMySQLUser(c *gin.Context) {
	ctx := c.Request.Context()
	id, ok := userID(c)
	if !ok {
		return
	}
	var req userReq
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	u := store.User{ID: id, Name: req.Name, Email: req.Email}
	if mysqlFailed(c, "update mysql user failed", id, store.UpdateMySQLUser(ctx, u)) {
		return
	}
	c.JSON(http.StatusOK, u)
}

func deleteMySQLUser(c *gin.Context) {
	id, ok := userID(c)
	if !ok {
		return
	}
	if mysqlFailed(c, "delete mysql user failed", id, store.DeleteMySQLUser(c.Request.Context(), id)) {
		return
	}
	c.Status(http.StatusNoContent)
}

// mysqlFailed err 非空时写好 404 / 500 并返回 true
func mysqlFailed(c *gin.Context, msg string, id int64, err error) bool {
	switch {
	case err == nil:
		return false
	case errors.Is(err, store.ErrNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": "user not found"})
	default:
		slog.ErrorContext(c.Request.Context(), msg, "user_id", id, "error", err)
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
	return true
}

// sleepMySQL 让 MySQL 服务端睡 ms 毫秒（默认 500），传的是请求的 ctx。
// 做完 200，失败 503；body 里都有 elapsed_ms
func sleepMySQL(c *gin.Context) {
	defer xmetric.TrackInFlight("mysql_sleep_inflight")()
	ctx := c.Request.Context()
	ms := queryInt(c, "ms", 500)
	start := time.Now()
	err := store.SleepMySQL(ctx, float64(ms)/1000)
	body := gin.H{"slept_ms": ms, "elapsed_ms": float64(time.Since(start).Microseconds()) / 1000}
	if err != nil {
		slog.WarnContext(ctx, "mysql sleep failed", "ms", ms, "error", err)
		body["error"] = err.Error()
		c.JSON(http.StatusServiceUnavailable, body)
		return
	}
	slog.InfoContext(ctx, "mysql sleep finished", "ms", ms)
	c.JSON(http.StatusOK, body)
}
