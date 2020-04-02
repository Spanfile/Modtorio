import React from 'react';
import { Button } from 'react-bootstrap';

export interface IWizardProps {
    initialStep?: number,
    done: () => void,
    children: React.ReactNode;
}

export interface IWizardStepProps {
    next: () => void,
    cancel: () => void,
}

export function Wizard(props: IWizardProps) {
    const [step, setStep] = React.useState(props.initialStep ?? 0);

    let childArray = React.Children.toArray(props.children);
    let maxSteps = childArray.length;
    let lastStep = maxSteps - 1;
    let currentStep = React.Children.toArray(props.children)[step];

    const nextStep = () => {
        if (step < lastStep) {
            return setStep(step + 1);
        }
    };

    return (
        <>
            {currentStep}
            <hr />
            <div className="d-flex flex-row-reverse">
                <Button
                    variant="primary"
                    className="ml-3"
                    onClick={nextStep}>
                    {step < lastStep ? "Next" : "Done"}
                </Button>
                <Button
                    variant="secondary"
                    className="ml-3">
                    Cancel
                </Button>
            </div>
        </>
    );
}
