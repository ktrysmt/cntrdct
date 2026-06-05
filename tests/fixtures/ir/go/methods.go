package store

// Conn holds a database handle.
type Conn struct {
	open bool
}

// Open marks the connection open.
func (c *Conn) Open(name string) error {
	c.connect(name)
	return nil
}

// Close marks the connection closed.
func (c *Conn) Close() {
	c.disconnect()
}
