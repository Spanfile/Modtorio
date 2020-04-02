import React from 'react';
import { Row, Col, Button, Dropdown, SplitButton } from 'react-bootstrap';

export function Control(props: { id: string | undefined }) {
    return (
        <>
            <Row xs={2} md={4} lg={6}>
                <Col className="pr-0">
                    <Button variant="outline-success" block disabled>Start</Button>
                </Col>
                <Col className="pr-0">
                    <SplitButton
                        key="shutdown"
                        id="shutdown"
                        variant="danger"
                        title="Shut down"
                    >
                        <Dropdown.Item eventKey="kill">Kill</Dropdown.Item>
                    </SplitButton>
                </Col>
                <Col className="pr-0">
                    <Button variant="warning" block>Restart</Button>
                </Col>
            </Row>
            <hr />
            <Row xs={2} md={4} lg={6}>
                <Col className="pr-0">
                    <Button variant="primary" block>Save</Button>
                </Col>
                <Col className="pr-0">
                    <Button variant="primary" block>Lua command</Button>
                </Col>
            </Row>
        </>
    );
}
