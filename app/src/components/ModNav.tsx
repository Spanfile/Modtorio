import React from 'react';
import { Row, Col, Navbar, Nav } from 'react-bootstrap';
import { NavLink } from 'react-router-dom';

export function ModNav() {
    return (
        <Row>
            <Col>
                <Navbar bg="dark" variant="dark">
                    <Navbar.Brand as={NavLink} to="/" exact>Modtorio</Navbar.Brand>
                    <Nav className="mr-auto">
                        <Nav.Link as={NavLink} to="/servers">Servers</Nav.Link>
                        <Nav.Link as={NavLink} to="/settings">Settings</Nav.Link>
                    </Nav>
                </Navbar>
            </Col>
        </Row>
    );
}